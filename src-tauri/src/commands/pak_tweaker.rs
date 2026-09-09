use std::fs::{self, File};
use std::io::Cursor;
use std::path::Path;
use serde::{Deserialize, Serialize};
use unreal_asset::{
    Asset,
    engine_version::EngineVersion,
    exports::{Export, ExportNormalTrait},
    properties::{Property, PropertyDataTrait},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PakTweakResult {
    pub success: bool,
    pub message: String,
    pub backup_created: bool,
    pub new_file_size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PakBackupStatus {
    pub has_backup: bool,
    pub backup_size_bytes: u64,
    pub backup_path: Option<String>,
}

/// Mutates a primitive property inside a .uasset/.uexp file within a .pak archive and repacks it
#[tauri::command]
pub async fn tweak_pak_property(
    _mod_id: Option<String>,
    pak_path: String,
    asset_internal_path: String,
    export_index: usize,
    property_name: String,
    new_value: serde_json::Value,
) -> Result<PakTweakResult, String> {
    let p_path = Path::new(&pak_path);
    if !p_path.exists() {
        return Err(format!("Pak file does not exist: {pak_path}"));
    }

    // 1. Safety Backup: Create .original.bak if not already present
    let bak_path = p_path.with_extension("pak.original.bak");
    let mut backup_created = false;
    if !bak_path.exists() {
        fs::copy(p_path, &bak_path)
            .map_err(|e| format!("Failed to create safety backup '{}': {e}", bak_path.display()))?;
        backup_created = true;
        crate::logger::log(&format!("pak_tweaker: Created safety backup at '{}'", bak_path.display()));
    }

    // 2. Normalize asset path
    let normalized_uasset = if asset_internal_path.to_lowercase().ends_with(".uexp") {
        format!("{}.uasset", &asset_internal_path[..asset_internal_path.len() - 5])
    } else if !asset_internal_path.to_lowercase().ends_with(".uasset") {
        format!("{}.uasset", asset_internal_path)
    } else {
        asset_internal_path.clone()
    };

    let normalized_uexp = format!("{}.uexp", &normalized_uasset[..normalized_uasset.len() - 7]);

    // 3. Read pak entries into memory
    let mut file = File::open(p_path)
        .map_err(|e| format!("Failed to open pak file: {e}"))?;
    let pak = repak::PakBuilder::new()
        .reader(&mut file)
        .map_err(|e| format!("Failed to read pak header: {e}"))?;

    let mount_point = pak.mount_point().to_string();
    let version = pak.version();
    let mut all_files = Vec::new();
    let mut target_uasset_bytes = None;
    let mut target_uexp_bytes = None;

    for entry_path in pak.files() {
        let mut entry_bytes = Vec::new();
        pak.read_file(&entry_path, &mut file, &mut entry_bytes)
            .map_err(|e| format!("Failed to read entry '{entry_path}': {e}"))?;

        if entry_path.eq_ignore_ascii_case(&normalized_uasset) {
            target_uasset_bytes = Some(entry_bytes);
        } else if entry_path.eq_ignore_ascii_case(&normalized_uexp) {
            target_uexp_bytes = Some(entry_bytes);
        } else {
            all_files.push((entry_path, entry_bytes));
        }
    }

    let uasset_data = target_uasset_bytes
        .ok_or_else(|| format!("Target .uasset '{normalized_uasset}' not found in pak archive"))?;

    // 4. Parse asset using unreal_asset
    let mut asset = Asset::new(
        Cursor::new(uasset_data),
        target_uexp_bytes.map(Cursor::new),
        EngineVersion::VER_UE5_1,
    ).map_err(|e| format!("Failed to parse uasset with unreal_asset: {e}"))?;

    // 5. Mutate the property
    let total_exports = asset.asset_data.exports.len();
    let export = asset.asset_data.exports.get_mut(export_index)
        .ok_or_else(|| format!("Export index {export_index} out of bounds (total exports: {total_exports})"))?;

    let normal = export.get_normal_export_mut()
        .ok_or_else(|| format!("Export #{export_index} is not a NormalExport"))?;

    let mut found_prop = false;
    for prop in &mut normal.properties {
        if prop.get_name().get_content() == property_name {
            mutate_property_value(prop, &new_value)?;
            found_prop = true;
            break;
        }
    }

    if !found_prop {
        return Err(format!("Property '{property_name}' not found in export #{export_index}"));
    }

    // 6. Serialize modified asset back to bytes
    let mut new_uasset_cursor = Cursor::new(Vec::new());
    let mut new_uexp_cursor = Cursor::new(Vec::new());
    asset.write_data(&mut new_uasset_cursor, Some(&mut new_uexp_cursor))
        .map_err(|e| format!("Failed to serialize modified uasset: {e}"))?;

    let new_uasset_bytes = new_uasset_cursor.into_inner();
    let new_uexp_bytes = new_uexp_cursor.into_inner();

    all_files.push((normalized_uasset.clone(), new_uasset_bytes));
    all_files.push((normalized_uexp.clone(), new_uexp_bytes));

    // Sort entries alphabetically for deterministic, canonical pak ordering
    all_files.sort_by(|a, b| a.0.cmp(&b.0));

    // 7. Write repacked pak file
    let temp_pak_path = p_path.with_extension("pak.tmp");
    {
        let mut out_file = File::create(&temp_pak_path)
            .map_err(|e| format!("Failed to create temporary pak file: {e}"))?;

        let mut pak_writer = repak::PakBuilder::new()
            .writer(&mut out_file, version, mount_point, None);

        for (rel_path, data) in &all_files {
            pak_writer
                .write_file(rel_path, false, data)
                .map_err(|e| format!("Failed to pack entry '{rel_path}': {e}"))?;
        }

        pak_writer
            .write_index()
            .map_err(|e| format!("Failed to finalize repak index: {e}"))?;
    }

    // Atomic replace
    drop(file);
    fs::rename(&temp_pak_path, p_path)
        .map_err(|e| format!("Failed to replace original pak with patched pak: {e}"))?;

    let new_file_size = fs::metadata(p_path)
        .map(|m| m.len())
        .unwrap_or(0);

    crate::logger::log(&format!(
        "pak_tweaker: Successfully patched property '{property_name}' in '{}' (new size: {} bytes)",
        p_path.display(),
        new_file_size
    ));

    Ok(PakTweakResult {
        success: true,
        message: format!("Successfully patched '{property_name}' in {}", p_path.file_name().unwrap_or_default().to_string_lossy()),
        backup_created,
        new_file_size_bytes: new_file_size,
    })
}

/// Mutates a single cell in a DataTable within a .pak archive and repacks it
#[tauri::command]
pub async fn tweak_datatable_cell(
    _mod_id: Option<String>,
    pak_path: String,
    asset_internal_path: String,
    row_name: String,
    column_name: String,
    new_value: serde_json::Value,
) -> Result<PakTweakResult, String> {
    let p_path = Path::new(&pak_path);
    if !p_path.exists() {
        return Err(format!("Pak file does not exist: {pak_path}"));
    }

    // Safety Backup
    let bak_path = p_path.with_extension("pak.original.bak");
    let mut backup_created = false;
    if !bak_path.exists() {
        fs::copy(p_path, &bak_path)
            .map_err(|e| format!("Failed to create safety backup: {e}"))?;
        backup_created = true;
    }

    let normalized_uasset = if asset_internal_path.to_lowercase().ends_with(".uexp") {
        format!("{}.uasset", &asset_internal_path[..asset_internal_path.len() - 5])
    } else if !asset_internal_path.to_lowercase().ends_with(".uasset") {
        format!("{}.uasset", asset_internal_path)
    } else {
        asset_internal_path.clone()
    };
    let normalized_uexp = format!("{}.uexp", &normalized_uasset[..normalized_uasset.len() - 7]);

    let mut file = File::open(p_path).map_err(|e| format!("Failed to open pak: {e}"))?;
    let pak = repak::PakBuilder::new().reader(&mut file).map_err(|e| format!("Failed to read pak: {e}"))?;
    let mount_point = pak.mount_point().to_string();
    let version = pak.version();

    let mut all_files = Vec::new();
    let mut target_uasset_bytes = None;
    let mut target_uexp_bytes = None;

    for entry_path in pak.files() {
        let mut entry_bytes = Vec::new();
        pak.read_file(&entry_path, &mut file, &mut entry_bytes).map_err(|e| format!("Read entry err: {e}"))?;
        if entry_path.eq_ignore_ascii_case(&normalized_uasset) {
            target_uasset_bytes = Some(entry_bytes);
        } else if entry_path.eq_ignore_ascii_case(&normalized_uexp) {
            target_uexp_bytes = Some(entry_bytes);
        } else {
            all_files.push((entry_path, entry_bytes));
        }
    }

    let uasset_data = target_uasset_bytes.ok_or_else(|| "Target .uasset not found".to_string())?;
    let mut asset = Asset::new(
        Cursor::new(uasset_data),
        target_uexp_bytes.map(Cursor::new),
        EngineVersion::VER_UE5_1,
    ).map_err(|e| format!("Parse uasset err: {e}"))?;

    let name_map = asset.get_name_map().get_ref().get_name_map_index_list().to_vec();
    let mut found_cell = false;
    for export in &mut asset.asset_data.exports {
        if let Export::DataTableExport(ref mut dt) = export {
            for row_struct in &mut dt.table.data {
                if row_struct.name.get_content() == row_name {
                    for prop in &mut row_struct.value {
                        if prop.get_name().get_content() == column_name {
                            mutate_property_value(prop, &new_value)?;
                            found_cell = true;
                            break;
                        }
                    }
                }
                if found_cell { break; }
            }
        } else if let Export::RawExport(ref mut r) = export {
            if r.data.len() >= 22 {
                let num_entries = i32::from_le_bytes(match r.data[10..14].try_into() {
                    Ok(b) => b,
                    Err(_) => continue,
                }) as usize;

                let struct_name = name_map.iter()
                    .find(|n| (n.ends_with("Data") || n.ends_with("MasterData")) && !n.contains('/'))
                    .cloned()
                    .unwrap_or_else(|| "TableRow".to_string());

                let schema_opt = crate::usmap::get_or_load_schema("");
                let (cols, prop_types, enum_types): (Vec<String>, Vec<String>, Vec<Option<String>>) = if let Some(ref sc) = schema_opt {
                    if let Some(st) = sc.find_struct(&struct_name) {
                        (
                            st.properties.iter().map(|p| p.name.clone()).collect(),
                            st.properties.iter().map(|p| p.type_name.clone()).collect(),
                            st.properties.iter().map(|p| p.enum_type.clone()).collect(),
                        )
                    } else {
                        (Vec::new(), Vec::new(), Vec::new())
                    }
                } else {
                    (Vec::new(), Vec::new(), Vec::new())
                };

                let mut cur = 14;
                for row_idx in 0..num_entries {
                    if cur + 8 > r.data.len() { break; }
                    let name_idx = i32::from_le_bytes(match r.data[cur..cur+4].try_into() {
                        Ok(b) => b,
                        Err(_) => break,
                    });
                    cur += 8;

                    let r_name = if name_idx >= 0 && (name_idx as usize) < name_map.len() {
                        name_map[name_idx as usize].clone()
                    } else {
                        format!("{}", row_idx + 1)
                    };

                    if cur + 2 > r.data.len() { break; }
                    let h_val = u16::from_le_bytes(match r.data[cur..cur+2].try_into() {
                        Ok(b) => b,
                        Err(_) => break,
                    });
                    let has_zeros = (h_val & 0x0080) != 0;
                    let val_num = ((h_val >> 9) & 0x7F) as usize;
                    cur += 2;

                    let mut z_mask = 0u8;
                    if has_zeros {
                        if cur >= r.data.len() { break; }
                        z_mask = r.data[cur];
                        cur += 1;
                    }

                    for p_idx in 0..val_num {
                        let col_name = cols.get(p_idx).cloned().unwrap_or_else(|| format!("Col_{}", p_idx + 1));
                        let p_type = prop_types.get(p_idx).map(|s| s.as_str()).unwrap_or("IntProperty");
                        let prop_size = match p_type {
                            "ByteProperty" | "BoolProperty" | "Int8Property" | "EnumProperty" => 1,
                            "Int16Property" | "UInt16Property" => 2,
                            "IntProperty" | "UInt32Property" | "FloatProperty" => 4,
                            "Int64Property" | "UInt64Property" | "DoubleProperty" | "NameProperty" => 8,
                            _ => 4,
                        };

                        let is_zero = if has_zeros && p_idx < 8 {
                            ((z_mask as u32) & (1u32 << p_idx)) != 0
                        } else {
                            false
                        };

                        if r_name == row_name && col_name == column_name {
                            if !is_zero && cur + prop_size <= r.data.len() {
                                match p_type {
                                    "ByteProperty" | "BoolProperty" | "Int8Property" | "EnumProperty" => {
                                        let b = if let Some(s) = new_value.as_str() {
                                            let clean = s.rsplit("::").next().unwrap_or(s);
                                            let mut found_idx = None;
                                            if let Some(ref sc) = schema_opt {
                                                if let Some(Some(ref e_name)) = enum_types.get(p_idx) {
                                                    if let Some(variants) = sc.get_enum(e_name) {
                                                        for (v_idx, v) in variants.iter().enumerate() {
                                                            if v.eq_ignore_ascii_case(clean) || v.eq_ignore_ascii_case(s) {
                                                                found_idx = Some(v_idx as u8);
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            found_idx.ok_or_else(|| format!("Unknown enum variant: {s}"))?
                                        } else {
                                            new_value.as_u64().ok_or_else(|| "Value must be an integer (0-255) or enum name".to_string())? as u8
                                        };
                                        r.data[cur] = b;
                                    }
                                    "FloatProperty" => {
                                        let f = new_value.as_f64().ok_or_else(|| "Value must be a float".to_string())? as f32;
                                        let b = f.to_bits().to_le_bytes();
                                        r.data[cur..cur+4].copy_from_slice(&b);
                                    }
                                    "Int16Property" => {
                                        let val = new_value.as_i64().ok_or_else(|| "Value must be an integer".to_string())? as i16;
                                        let b = val.to_le_bytes();
                                        r.data[cur..cur+2].copy_from_slice(&b);
                                    }
                                    _ => {
                                        let i = new_value.as_i64().ok_or_else(|| "Value must be an integer".to_string())? as i32;
                                        let b = i.to_le_bytes();
                                        r.data[cur..cur+4].copy_from_slice(&b);
                                    }
                                }
                                found_cell = true;
                                break;
                            } else if is_zero {
                                return Err(format!("Cannot directly edit cell '{column_name}' in row '{row_name}' because it currently uses default zero-suppression in this binary package."));
                            }
                        }

                        if !is_zero {
                            cur += prop_size;
                        }
                    }
                    if found_cell { break; }
                }
            }
        }
        if found_cell { break; }
    }

    if !found_cell {
        return Err(format!("Cell [Row: '{row_name}', Col: '{column_name}'] not found in DataTable"));
    }

    let mut new_uasset_cursor = Cursor::new(Vec::new());
    let mut new_uexp_cursor = Cursor::new(Vec::new());
    asset.write_data(&mut new_uasset_cursor, Some(&mut new_uexp_cursor))
        .map_err(|e| format!("Serialize error: {e}"))?;

    all_files.push((normalized_uasset, new_uasset_cursor.into_inner()));
    all_files.push((normalized_uexp, new_uexp_cursor.into_inner()));
    all_files.sort_by(|a, b| a.0.cmp(&b.0));

    let temp_pak_path = p_path.with_extension("pak.tmp");
    {
        let mut out_file = File::create(&temp_pak_path).map_err(|e| format!("Create temp pak err: {e}"))?;
        let mut pak_writer = repak::PakBuilder::new().writer(&mut out_file, version, mount_point, None);
        for (rel_path, data) in &all_files {
            pak_writer.write_file(rel_path, false, data).map_err(|e| format!("Pack err: {e}"))?;
        }
        pak_writer.write_index().map_err(|e| format!("Index err: {e}"))?;
    }

    drop(file);
    fs::rename(&temp_pak_path, p_path).map_err(|e| format!("Rename pak err: {e}"))?;

    let new_size = fs::metadata(p_path).map(|m| m.len()).unwrap_or(0);
    Ok(PakTweakResult {
        success: true,
        message: format!("Successfully updated [{row_name}.{column_name}] in DataTable!"),
        backup_created,
        new_file_size_bytes: new_size,
    })
}

/// Reverts a patched .pak file back to its authentic pre-modification .original.bak backup
#[tauri::command]
pub async fn revert_pak_to_backup(
    _mod_id: Option<String>,
    pak_path: String,
) -> Result<PakTweakResult, String> {
    let p_path = Path::new(&pak_path);
    let bak_path = p_path.with_extension("pak.original.bak");

    if !bak_path.exists() {
        return Err(format!("No safety backup found for '{}'", p_path.display()));
    }

    fs::copy(&bak_path, p_path)
        .map_err(|e| format!("Failed to restore backup over original pak: {e}"))?;

    // Optionally delete backup after successful revert
    let _ = fs::remove_file(&bak_path);

    let restored_size = fs::metadata(p_path).map(|m| m.len()).unwrap_or(0);
    crate::logger::log(&format!("pak_tweaker: Reverted '{}' from backup", p_path.display()));

    Ok(PakTweakResult {
        success: true,
        message: format!("Restored {} from original backup.", p_path.file_name().unwrap_or_default().to_string_lossy()),
        backup_created: false,
        new_file_size_bytes: restored_size,
    })
}

/// Queries whether an authentic .original.bak file exists for a .pak archive
#[tauri::command]
pub async fn get_pak_backup_status(
    _mod_id: Option<String>,
    pak_path: String,
) -> Result<PakBackupStatus, String> {
    let p_path = Path::new(&pak_path);
    let bak_path = p_path.with_extension("pak.original.bak");

    if bak_path.exists() {
        let size = fs::metadata(&bak_path).map(|m| m.len()).unwrap_or(0);
        Ok(PakBackupStatus {
            has_backup: true,
            backup_size_bytes: size,
            backup_path: Some(bak_path.to_string_lossy().to_string()),
        })
    } else {
        Ok(PakBackupStatus {
            has_backup: false,
            backup_size_bytes: 0,
            backup_path: None,
        })
    }
}

/// Helper mutating an unreal_asset Property from a JSON Value
fn mutate_property_value(prop: &mut Property, new_val: &serde_json::Value) -> Result<(), String> {
    match prop {
        Property::IntProperty(p) => {
            p.value = new_val.as_i64().ok_or_else(|| "Expected integer for IntProperty".to_string())? as i32;
        }
        Property::Int8Property(p) => {
            p.value = new_val.as_i64().ok_or_else(|| "Expected integer for Int8Property".to_string())? as i8;
        }
        Property::Int16Property(p) => {
            p.value = new_val.as_i64().ok_or_else(|| "Expected integer for Int16Property".to_string())? as i16;
        }
        Property::Int64Property(p) => {
            p.value = new_val.as_i64().ok_or_else(|| "Expected integer for Int64Property".to_string())?;
        }
        Property::UInt16Property(p) => {
            p.value = new_val.as_u64().ok_or_else(|| "Expected unsigned integer for UInt16Property".to_string())? as u16;
        }
        Property::UInt32Property(p) => {
            p.value = new_val.as_u64().ok_or_else(|| "Expected unsigned integer for UInt32Property".to_string())? as u32;
        }
        Property::UInt64Property(p) => {
            p.value = new_val.as_u64().ok_or_else(|| "Expected unsigned integer for UInt64Property".to_string())?;
        }
        Property::FloatProperty(p) => {
            let f = new_val.as_f64().ok_or_else(|| "Expected number for FloatProperty".to_string())? as f32;
            p.value.0 = f;
        }
        Property::DoubleProperty(p) => {
            let d = new_val.as_f64().ok_or_else(|| "Expected number for DoubleProperty".to_string())?;
            p.value.0 = d;
        }
        Property::BoolProperty(p) => {
            p.value = new_val.as_bool().ok_or_else(|| "Expected boolean for BoolProperty".to_string())?;
        }
        Property::StrProperty(p) => {
            let s = new_val.as_str().ok_or_else(|| "Expected string for StrProperty".to_string())?;
            p.value = Some(s.to_string());
        }
        Property::ByteProperty(p) => {
            if let unreal_asset::properties::int_property::BytePropertyValue::Byte(ref mut b) = p.value {
                *b = new_val.as_u64().ok_or_else(|| "Expected byte value (0-255)".to_string())? as u8;
            }
        }
        _ => {
            return Err(format!("Property type '{:?}' is not currently editable as a primitive scalar", prop));
        }
    }

    Ok(())
}

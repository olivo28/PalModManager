use std::collections::HashMap;
use unreal_asset::properties::{Property, PropertyDataTrait};
use unreal_asset::exports::{Export, ExportNormalTrait, ExportBaseTrait};
use unreal_asset::properties::int_property::BytePropertyValue;
use super::types::{UAssetLiveProperty, UAssetDataTableGrid, UAssetDataTableRow};

/// Converts an unreal_asset Property into a serialized JSON Value and human-readable string
pub fn property_to_json_and_display(prop: &Property) -> (serde_json::Value, String, bool, Option<String>, Option<String>) {
    match prop {
        Property::IntProperty(p) => {
            (serde_json::json!(p.value), p.value.to_string(), true, None, None)
        }
        Property::Int8Property(p) => {
            (serde_json::json!(p.value), p.value.to_string(), true, None, None)
        }
        Property::Int16Property(p) => {
            (serde_json::json!(p.value), p.value.to_string(), true, None, None)
        }
        Property::Int64Property(p) => {
            (serde_json::json!(p.value), p.value.to_string(), true, None, None)
        }
        Property::UInt16Property(p) => {
            (serde_json::json!(p.value), p.value.to_string(), true, None, None)
        }
        Property::UInt32Property(p) => {
            (serde_json::json!(p.value), p.value.to_string(), true, None, None)
        }
        Property::UInt64Property(p) => {
            (serde_json::json!(p.value), p.value.to_string(), true, None, None)
        }
        Property::FloatProperty(p) => {
            let val = p.value.0;
            (serde_json::json!(val), format!("{:.2}", val), true, None, None)
        }
        Property::DoubleProperty(p) => {
            let val = p.value.0;
            (serde_json::json!(val), format!("{:.4}", val), true, None, None)
        }
        Property::BoolProperty(p) => {
            (serde_json::json!(p.value), p.value.to_string(), true, None, None)
        }
        Property::StrProperty(p) => {
            let s = p.value.clone().unwrap_or_default();
            (serde_json::json!(s), format!("\"{}\"", s), true, None, None)
        }
        Property::NameProperty(p) => {
            let name = p.value.get_content();
            (serde_json::json!(name), name.clone(), true, None, None)
        }
        Property::TextProperty(p) => {
            let txt = p.culture_invariant_string.clone().unwrap_or_else(|| "Text".to_string());
            (serde_json::json!(txt), format!("\"{}\"", txt), false, None, None)
        }
        Property::ByteProperty(p) => {
            match &p.value {
                BytePropertyValue::Byte(b) => {
                    (serde_json::json!(b), b.to_string(), true, None, None)
                }
                BytePropertyValue::FName(f) => {
                    let val = f.get_content();
                    let enum_name = p.enum_type.as_ref().map(|e| e.get_content());
                    let display = if let Some(ref e) = enum_name {
                        format!("{}::{}", e, val)
                    } else {
                        val.clone()
                    };
                    (serde_json::json!(val), display, true, None, enum_name)
                }
            }
        }
        Property::EnumProperty(p) => {
            let val = p.value.as_ref().map(|v| v.get_content()).unwrap_or_default();
            let enum_type = p.enum_type.as_ref().map(|e| e.get_content()).unwrap_or_default();
            (serde_json::json!(val), format!("{}::{}", enum_type, val), true, None, Some(enum_type))
        }
        Property::StructProperty(p) => {
            let struct_type = p.struct_type.as_ref().map(|s| s.get_content()).unwrap_or_else(|| "Struct".to_string());
            let count = p.value.len();
            (serde_json::json!({ "_type": &struct_type, "_fields": count }), format!("{} ({} fields)", struct_type, count), false, Some(struct_type), None)
        }
        Property::ArrayProperty(p) => {
            let count = p.value.len();
            (serde_json::json!({ "_array_len": count }), format!("Array[{}]", count), false, None, None)
        }
        Property::ObjectProperty(p) => {
            (serde_json::json!(p.value.index), format!("ObjectRef({})", p.value.index), false, None, None)
        }
        _ => {
            let type_name = prop.get_name().get_content();
            (serde_json::json!(type_name), type_name.clone(), false, None, None)
        }
    }
}

/// Extracts all instantiated properties from all exports in the asset
pub fn extract_instantiated_properties(exports: &[Export]) -> Vec<UAssetLiveProperty> {
    let mut results = Vec::new();

    for (export_idx, export) in exports.iter().enumerate() {
        if let Some(normal) = export.get_normal_export() {
            for prop in &normal.properties {
                let name = prop.get_name().get_content();
                let prop_type = format!("{:?}", prop);
                // Clean type name from enum debug representation (e.g. "IntProperty(...)")
                let clean_type = prop_type.split('(').next().unwrap_or(&prop_type).to_string();

                let (value, raw_value_display, is_editable, struct_type, enum_value) = property_to_json_and_display(prop);

                results.push(UAssetLiveProperty {
                    export_index: export_idx,
                    name,
                    property_type: clean_type,
                    value,
                    raw_value_display,
                    is_editable,
                    struct_type,
                    enum_value,
                    vanilla_default_display: None,
                    is_delta: true,
                });
            }
        }
    }

    results
}

/// Decodes unversioned properties from cooked Blueprint CDO / Component RawExports using the USMAP schema
pub fn extract_unversioned_cdo_properties(
    exports: &[Export],
    imports: &[super::types::UAssetImportItem],
    _name_map: &[String],
    asset_name: &str,
) -> Vec<UAssetLiveProperty> {
    let mut results = Vec::new();
    let schema_opt = crate::usmap::get_or_load_schema("");
    let sc = match schema_opt {
        Some(ref s) => s,
        None => return results,
    };

    for (export_idx, export) in exports.iter().enumerate() {
        let r = match export {
            Export::RawExport(raw) => raw,
            _ => continue,
        };

        if r.data.len() < 2 {
            continue;
        }

        let base = export.get_base_export();
        let obj_name = base.object_name.get_content();

        // Determine class name from import table if available
        let class_name = if base.class_index.index < 0 {
            let import_idx = (-base.class_index.index - 1) as usize;
            imports.get(import_idx).map(|imp| imp.object_name.clone()).unwrap_or_default()
        } else {
            String::new()
        };

        // Candidate struct names in USMAP
        let mut candidates = Vec::new();
        if !class_name.is_empty() {
            candidates.push(class_name.clone());
        }
        if obj_name.ends_with("_GEN_VARIABLE") {
            let prefix = &obj_name[..obj_name.len() - 13];
            candidates.push(format!("{}Component", prefix));
            candidates.push(prefix.to_string());
        }
        if obj_name.starts_with("Default__") {
            let inner = &obj_name[9..];
            let clean = if inner.ends_with("_C") { &inner[..inner.len() - 2] } else { inner };
            candidates.push(clean.to_string());
            if clean.starts_with("BP_") {
                candidates.push(clean[3..].to_string());
            }
        }
        let clean_asset = asset_name.strip_suffix(".uasset").or_else(|| asset_name.strip_suffix(".uexp")).unwrap_or(asset_name);
        let base_asset = clean_asset.split('/').last().unwrap_or(clean_asset);
        if base_asset.starts_with("BP_") {
            candidates.push(base_asset[3..].to_string());
        }
        candidates.push(base_asset.to_string());

        let mut matched_struct = None;
        for c in candidates {
            if let Some(st) = sc.find_struct(&c) {
                matched_struct = Some(st);
                break;
            }
        }

        let st = match matched_struct {
            Some(s) => s,
            None => continue,
        };

        // Parse FUnversionedHeader fragments
        let mut cur = 0;
        let mut fragments = Vec::new();
        let mut is_last = false;
        while !is_last && cur + 2 <= r.data.len() {
            let val = u16::from_le_bytes([r.data[cur], r.data[cur + 1]]);
            cur += 2;
            let skip_num = (val & 0x007F) as usize;
            let has_zeros = (val & 0x0080) != 0;
            let value_num = ((val >> 9) & 0x7F) as usize;
            is_last = (val & 0x0100) != 0;
            fragments.push((skip_num, has_zeros, value_num));
        }

        if fragments.is_empty() {
            continue;
        }

        let mut total_zeros = 0;
        for (_, has_zeros, value_num) in &fragments {
            if *has_zeros {
                total_zeros += *value_num;
            }
        }
        let zero_mask_len = (total_zeros + 7) / 8;
        let zero_mask = if zero_mask_len > 0 && cur + zero_mask_len <= r.data.len() {
            let mask_bytes = r.data[cur..cur + zero_mask_len].to_vec();
            cur += zero_mask_len;
            mask_bytes
        } else {
            Vec::new()
        };

        let mut prop_idx = 0;
        let mut zero_bit_idx = 0;

        for (skip_num, has_zeros, value_num) in fragments {
            prop_idx += skip_num;
            for _ in 0..value_num {
                let is_zero = if has_zeros {
                    let byte_idx = zero_bit_idx / 8;
                    let bit_idx = zero_bit_idx % 8;
                    zero_bit_idx += 1;
                    if byte_idx < zero_mask.len() {
                        (zero_mask[byte_idx] & (1 << bit_idx)) != 0
                    } else {
                        false
                    }
                } else {
                    false
                };

                if let Some(prop_meta) = st.properties.get(prop_idx) {
                    let p_type = prop_meta.type_name.as_str();
                    let (val_json, display_str, is_editable) = if is_zero {
                        (serde_json::json!(0), "0".to_string(), true)
                    } else {
                        match p_type {
                            "IntProperty" => {
                                if cur + 4 <= r.data.len() {
                                    let v = i32::from_le_bytes(r.data[cur..cur + 4].try_into().unwrap());
                                    cur += 4;
                                    (serde_json::json!(v), v.to_string(), true)
                                } else {
                                    (serde_json::Value::Null, String::new(), false)
                                }
                            }
                            "Int64Property" => {
                                if cur + 8 <= r.data.len() {
                                    let v = i64::from_le_bytes(r.data[cur..cur + 8].try_into().unwrap());
                                    cur += 8;
                                    (serde_json::json!(v), v.to_string(), true)
                                } else {
                                    (serde_json::Value::Null, String::new(), false)
                                }
                            }
                            "FloatProperty" => {
                                if cur + 4 <= r.data.len() {
                                    let v = f32::from_le_bytes(r.data[cur..cur + 4].try_into().unwrap());
                                    cur += 4;
                                    (serde_json::json!(v), format!("{:.2}", v), true)
                                } else {
                                    (serde_json::Value::Null, String::new(), false)
                                }
                            }
                            "DoubleProperty" => {
                                if cur + 8 <= r.data.len() {
                                    let v = f64::from_le_bytes(r.data[cur..cur + 8].try_into().unwrap());
                                    cur += 8;
                                    (serde_json::json!(v), format!("{:.4}", v), true)
                                } else {
                                    (serde_json::Value::Null, String::new(), false)
                                }
                            }
                            "BoolProperty" => {
                                if cur + 1 <= r.data.len() {
                                    let v = r.data[cur] != 0;
                                    cur += 1;
                                    (serde_json::json!(v), v.to_string(), true)
                                } else {
                                    (serde_json::Value::Null, String::new(), false)
                                }
                            }
                            "ByteProperty" => {
                                if cur + 1 <= r.data.len() {
                                    let v = r.data[cur];
                                    cur += 1;
                                    (serde_json::json!(v), v.to_string(), true)
                                } else {
                                    (serde_json::Value::Null, String::new(), false)
                                }
                            }
                            "EnumProperty" => {
                                if cur + 1 <= r.data.len() {
                                    let b = r.data[cur];
                                    cur += 1;
                                    let enum_label = if let Some(ref e_name) = prop_meta.enum_type {
                                        if let Some(variants) = sc.get_enum(e_name) {
                                            if (b as usize) < variants.len() {
                                                format!("{}::{}", e_name, variants[b as usize])
                                            } else {
                                                format!("{} ({})", b, e_name)
                                            }
                                        } else {
                                            b.to_string()
                                        }
                                    } else {
                                        b.to_string()
                                    };
                                    (serde_json::json!(enum_label), enum_label, true)
                                } else {
                                    (serde_json::Value::Null, String::new(), false)
                                }
                            }
                            _ => (serde_json::json!(format!("<{}>", p_type)), format!("<{}>", p_type), false),
                        }
                    };

                    if !display_str.is_empty() {
                        // Compute vanilla baseline display if property is known
                        let vanilla_default = match prop_meta.name.as_str() {
                            "SlotNum" => {
                                if base_asset.contains("ItemChest_01") { Some("16".to_string()) }
                                else if base_asset.contains("ItemChest_02") { Some("32".to_string()) }
                                else if base_asset.contains("ItemChest_03") { Some("48".to_string()) }
                                else if base_asset.contains("ItemChest_04") { Some("64".to_string()) }
                                else { None }
                            }
                            "CharacterRankUpRequiredNumDefault" => Some("4".to_string()),
                            _ => None,
                        };

                        results.push(UAssetLiveProperty {
                            export_index: export_idx,
                            name: format!("{} ({})", prop_meta.name, obj_name),
                            property_type: p_type.to_string(),
                            value: val_json,
                            raw_value_display: display_str,
                            is_editable,
                            struct_type: prop_meta.struct_type.clone(),
                            enum_value: prop_meta.enum_type.clone(),
                            vanilla_default_display: vanilla_default,
                            is_delta: true,
                        });
                    }
                }
                prop_idx += 1;
            }
        }
    }

    results
}

/// Extracts a structured DataTable grid if the asset contains a DataTableExport or a cooked unversioned RawExport DataTable
pub fn extract_datatable_grid(exports: &[Export], name_map: &[String], asset_name: &str) -> Option<UAssetDataTableGrid> {
    // 1. Standard unreal_asset DataTableExport branch
    for export in exports {
        if let Export::DataTableExport(dt) = export {
            let mut columns_set = std::collections::BTreeSet::new();
            let mut rows = Vec::new();
            let mut detected_struct_name = String::new();

            for row_struct in &dt.table.data {
                let row_name = row_struct.name.get_content();
                if detected_struct_name.is_empty() {
                    if let Some(ref st) = row_struct.struct_type {
                        detected_struct_name = st.get_content();
                    }
                }

                let mut row_values = HashMap::new();
                for prop in &row_struct.value {
                    let col_name = prop.get_name().get_content();
                    columns_set.insert(col_name.clone());
                    let (val, _, _, _, _) = property_to_json_and_display(prop);
                    row_values.insert(col_name, val);
                }

                rows.push(UAssetDataTableRow {
                    row_name,
                    values: row_values,
                });
            }

            let columns: Vec<String> = columns_set.into_iter().collect();
            let total_rows = rows.len();

            return Some(UAssetDataTableGrid {
                row_struct_name: if detected_struct_name.is_empty() { "TableRow".to_string() } else { detected_struct_name },
                columns,
                rows,
                total_rows,
            });
        }
    }

    // 2. High-Performance Cooked/Unversioned RawExport DataTable branch
    let is_datatable_asset = asset_name.starts_with("DT_") 
        || asset_name.ends_with("DataTable.uasset") 
        || asset_name.contains("/DataTable/")
        || exports.iter().any(|e| {
            let base = e.get_base_export();
            let obj = base.object_name.get_content();
            obj.starts_with("DT_") || obj.ends_with("DataTable")
        });

    if is_datatable_asset {
        for export in exports {
            if let Export::RawExport(r) = export {
                if r.data.len() >= 22 {
                    let num_entries = i32::from_le_bytes(match r.data[10..14].try_into() {
                        Ok(b) => b,
                        Err(_) => continue,
                    }) as usize;

                    if num_entries > 0 && num_entries <= 50_000 {
                        // Discover struct name from NameMap or USMAP
                        let struct_name = name_map.iter()
                            .find(|n| (n.ends_with("Data") || n.ends_with("MasterData")) && !n.contains('/'))
                            .cloned()
                            .unwrap_or_else(|| "TableRow".to_string());

                        // Query columns from active USMAP schema if available
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
                        let mut rows = Vec::new();
                        let mut detected_cols = std::collections::BTreeSet::new();

                        for row_idx in 0..num_entries {
                            if cur + 8 > r.data.len() {
                                break;
                            }
                            let name_idx = i32::from_le_bytes(match r.data[cur..cur+4].try_into() {
                                Ok(b) => b,
                                Err(_) => break,
                            });
                            cur += 8;

                            let row_name = if name_idx >= 0 && (name_idx as usize) < name_map.len() {
                                name_map[name_idx as usize].clone()
                            } else {
                                format!("{}", row_idx + 1)
                            };

                            if cur + 2 > r.data.len() {
                                break;
                            }
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

                            let mut row_values = HashMap::new();
                            for p_idx in 0..val_num {
                                let col_name = cols.get(p_idx).cloned().unwrap_or_else(|| format!("Col_{}", p_idx + 1));
                                detected_cols.insert(col_name.clone());

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

                                if is_zero {
                                    let mut resolved_zero = None;
                                    if p_type == "EnumProperty" {
                                        if let Some(ref sc) = schema_opt {
                                            if let Some(Some(ref e_name)) = enum_types.get(p_idx) {
                                                if let Some(variants) = sc.get_enum(e_name) {
                                                    if !variants.is_empty() {
                                                        resolved_zero = Some(format!("{}::{}", e_name, variants[0]));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if let Some(s) = resolved_zero {
                                        row_values.insert(col_name, serde_json::json!(s));
                                    } else {
                                        row_values.insert(col_name, serde_json::json!(0));
                                    }
                                } else if cur + prop_size <= r.data.len() {
                                    let val_json = match p_type {
                                        "ByteProperty" | "BoolProperty" | "Int8Property" | "EnumProperty" => {
                                            let b = r.data[cur];
                                            cur += 1;
                                            let mut resolved_enum = None;
                                            if p_type == "EnumProperty" {
                                                if let Some(ref sc) = schema_opt {
                                                    if let Some(Some(ref e_name)) = enum_types.get(p_idx) {
                                                        if let Some(variants) = sc.get_enum(e_name) {
                                                            if (b as usize) < variants.len() {
                                                                resolved_enum = Some(format!("{}::{}", e_name, variants[b as usize]));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            if let Some(s) = resolved_enum {
                                                serde_json::json!(s)
                                            } else {
                                                serde_json::json!(b)
                                            }
                                        }
                                        "Int16Property" => {
                                            let val = i16::from_le_bytes(match r.data[cur..cur+2].try_into() {
                                                Ok(b) => b,
                                                Err(_) => break,
                                            });
                                            cur += 2;
                                            serde_json::json!(val)
                                        }
                                        "UInt16Property" => {
                                            let val = u16::from_le_bytes(match r.data[cur..cur+2].try_into() {
                                                Ok(b) => b,
                                                Err(_) => break,
                                            });
                                            cur += 2;
                                            serde_json::json!(val)
                                        }
                                        "FloatProperty" => {
                                            let raw = u32::from_le_bytes(match r.data[cur..cur+4].try_into() {
                                                Ok(b) => b,
                                                Err(_) => break,
                                            });
                                            cur += 4;
                                            let f = f32::from_bits(raw);
                                            serde_json::json!(f)
                                        }
                                        "DoubleProperty" => {
                                            let raw = u64::from_le_bytes(match r.data[cur..cur+8].try_into() {
                                                Ok(b) => b,
                                                Err(_) => break,
                                            });
                                            cur += 8;
                                            let d = f64::from_bits(raw);
                                            serde_json::json!(d)
                                        }
                                        "Int64Property" => {
                                            let val = i64::from_le_bytes(match r.data[cur..cur+8].try_into() {
                                                Ok(b) => b,
                                                Err(_) => break,
                                            });
                                            cur += 8;
                                            serde_json::json!(val)
                                        }
                                        "UInt64Property" => {
                                            let val = u64::from_le_bytes(match r.data[cur..cur+8].try_into() {
                                                Ok(b) => b,
                                                Err(_) => break,
                                            });
                                            cur += 8;
                                            serde_json::json!(val)
                                        }
                                        "UInt32Property" => {
                                            let val = u32::from_le_bytes(match r.data[cur..cur+4].try_into() {
                                                Ok(b) => b,
                                                Err(_) => break,
                                            });
                                            cur += 4;
                                            serde_json::json!(val)
                                        }
                                        "NameProperty" => {
                                            let n_idx = i32::from_le_bytes(match r.data[cur..cur+4].try_into() {
                                                Ok(b) => b,
                                                Err(_) => break,
                                            });
                                            cur += 8;
                                            if n_idx >= 0 && (n_idx as usize) < name_map.len() {
                                                serde_json::json!(name_map[n_idx as usize])
                                            } else {
                                                serde_json::json!(n_idx)
                                            }
                                        }
                                        _ => {
                                            let i = i32::from_le_bytes(match r.data[cur..cur+4].try_into() {
                                                Ok(b) => b,
                                                Err(_) => break,
                                            });
                                            cur += 4;
                                            serde_json::json!(i)
                                        }
                                    };
                                    row_values.insert(col_name, val_json);
                                }
                            }

                            rows.push(UAssetDataTableRow {
                                row_name,
                                values: row_values,
                            });
                        }

                        if !rows.is_empty() {
                            let final_columns = if !cols.is_empty() {
                                cols
                            } else {
                                detected_cols.into_iter().collect()
                            };
                            let total_rows = rows.len();

                            return Some(UAssetDataTableGrid {
                                row_struct_name: struct_name,
                                columns: final_columns,
                                rows,
                                total_rows,
                            });
                        }
                    }
                }
            }
        }
    }

    None
}

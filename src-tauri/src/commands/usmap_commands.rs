use tauri::State;
use crate::state::AppState;
use crate::usmap::{UsmapStatus, UsmapStruct, UsmapProperty};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsmapSearchItem {
    pub name: String,
    pub category: String, // "struct" | "enum" | "name"
    pub super_type: Option<String>,
    pub property_count: usize,
    pub enum_values_count: usize,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsmapSearchResult {
    pub total_items: usize,
    pub page: usize,
    pub page_size: usize,
    pub items: Vec<UsmapSearchItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsmapStructFullDetails {
    pub name: String,
    pub super_type: Option<String>,
    pub inheritance_chain: Vec<String>,
    pub properties: Vec<UsmapProperty>,
    pub total_properties_with_ancestors: usize,
}

#[tauri::command]
pub fn get_mappings_status(state: State<'_, AppState>) -> Result<UsmapStatus, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let game_path = data.settings.game_path.clone();
    drop(data);

    Ok(crate::usmap::get_mappings_status(&program_path, &game_path))
}

#[tauri::command]
pub async fn sync_mappings_now(state: State<'_, AppState>) -> Result<UsmapStatus, String> {
    let (program_path, game_path) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        (data.settings.program_path.clone(), data.settings.game_path.clone())
    };

    crate::usmap::invalidate_schema_cache();
    crate::usmap::sync_mappings_async(program_path, game_path).await
}

#[tauri::command]
pub fn get_usmap_struct_info(struct_name: String, state: State<'_, AppState>) -> Result<Option<UsmapStruct>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    drop(data);

    if let Some(schema) = crate::usmap::get_or_load_schema(&program_path) {
        Ok(schema.get_struct(&struct_name).cloned())
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn get_usmap_full_struct_details(name: String, state: State<'_, AppState>) -> Result<Option<UsmapStructFullDetails>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    drop(data);

    if let Some(schema) = crate::usmap::get_or_load_schema(&program_path) {
        if let Some(s) = schema.find_struct(&name) {
            let mut inheritance_chain = Vec::new();
            inheritance_chain.push(s.name.clone());

            let mut current_super = s.super_type.clone();
            let mut total_props = s.properties.len();

            while let Some(parent_name) = current_super {
                if let Some(parent_struct) = schema.get_struct(&parent_name) {
                    inheritance_chain.push(parent_struct.name.clone());
                    total_props += parent_struct.properties.len();
                    current_super = parent_struct.super_type.clone();
                } else {
                    inheritance_chain.push(parent_name);
                    break;
                }
            }

            return Ok(Some(UsmapStructFullDetails {
                name: s.name.clone(),
                super_type: s.super_type.clone(),
                inheritance_chain,
                properties: s.properties.clone(),
                total_properties_with_ancestors: total_props,
            }));
        }
    }
    Ok(None)
}

#[tauri::command]
pub fn search_usmap_entries(
    query: String,
    filter_type: String, // "all" | "structs" | "enums" | "names"
    page: usize,
    page_size: usize,
    state: State<'_, AppState>,
) -> Result<UsmapSearchResult, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    drop(data);

    let schema = match crate::usmap::get_or_load_schema(&program_path) {
        Some(s) => s,
        None => {
            return Ok(UsmapSearchResult {
                total_items: 0,
                page,
                page_size,
                items: Vec::new(),
            })
        }
    };

    let q = query.trim().to_lowercase();
    let mut matches = Vec::new();

    // 1. Structs / Classes
    if filter_type == "all" || filter_type == "structs" {
        for (name, s) in &schema.structs {
            if q.is_empty() || name.to_lowercase().contains(&q) || s.super_type.as_ref().map_or(false, |st| st.to_lowercase().contains(&q)) {
                let preview = if let Some(ref st) = s.super_type {
                    format!("Super: {} ({} properties)", st, s.properties.len())
                } else {
                    format!("{} properties", s.properties.len())
                };

                matches.push(UsmapSearchItem {
                    name: name.clone(),
                    category: "struct".to_string(),
                    super_type: s.super_type.clone(),
                    property_count: s.properties.len(),
                    enum_values_count: 0,
                    preview,
                });
            }
        }
    }

    // 2. Enums
    if filter_type == "all" || filter_type == "enums" {
        for (name, vals) in &schema.enums {
            if q.is_empty() || name.to_lowercase().contains(&q) || vals.iter().any(|v| v.to_lowercase().contains(&q)) {
                let preview = format!("{} values: {}", vals.len(), vals.iter().take(3).cloned().collect::<Vec<_>>().join(", "));
                matches.push(UsmapSearchItem {
                    name: name.clone(),
                    category: "enum".to_string(),
                    super_type: None,
                    property_count: 0,
                    enum_values_count: vals.len(),
                    preview,
                });
            }
        }
    }

    // 3. FNames
    if filter_type == "names" {
        for name in &schema.names {
            if q.is_empty() || name.to_lowercase().contains(&q) {
                matches.push(UsmapSearchItem {
                    name: name.clone(),
                    category: "name".to_string(),
                    super_type: None,
                    property_count: 0,
                    enum_values_count: 0,
                    preview: "FName Entry".to_string(),
                });
            }
        }
    }

    // Sort: exact matches first, then alphabetical
    matches.sort_by(|a, b| {
        let a_exact = a.name.eq_ignore_ascii_case(&q);
        let b_exact = b.name.eq_ignore_ascii_case(&q);
        if a_exact && !b_exact {
            std::cmp::Ordering::Less
        } else if !a_exact && b_exact {
            std::cmp::Ordering::Greater
        } else {
            a.name.cmp(&b.name)
        }
    });

    let total_items = matches.len();
    let start_idx = page.saturating_mul(page_size);
    let end_idx = (start_idx + page_size).min(total_items);

    let items = if start_idx < total_items {
        matches[start_idx..end_idx].to_vec()
    } else {
        Vec::new()
    };

    Ok(UsmapSearchResult {
        total_items,
        page,
        page_size,
        items,
    })
}

#[tauri::command]
pub fn get_usmap_enum_info(enum_name: String, state: State<'_, AppState>) -> Result<Option<Vec<String>>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    drop(data);

    if let Some(schema) = crate::usmap::get_or_load_schema(&program_path) {
        Ok(schema.get_enum(&enum_name).cloned())
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn get_usmap_summary(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let game_path = data.settings.game_path.clone();
    drop(data);

    let status = crate::usmap::get_mappings_status(&program_path, &game_path);
    let usmap_path = crate::usmap::sync::get_active_usmap_path(&program_path)
        .to_string_lossy()
        .to_string();

    if let Some(schema) = crate::usmap::get_or_load_schema(&program_path) {
        Ok(serde_json::json!({
            "loaded": true,
            "totalStructs": schema.total_structs,
            "totalEnums": schema.total_enums,
            "totalNames": schema.total_names,
            "gameVersion": schema.game_version,
            "path": usmap_path,
            "sha256": status.local_sha256,
            "gameBuild": status.installed_build.build_id.or(status.installed_build.game_version)
        }))
    } else {
        Ok(serde_json::json!({
            "loaded": false,
            "totalStructs": 0,
            "totalEnums": 0,
            "totalNames": 0,
            "path": usmap_path,
            "gameBuild": status.installed_build.build_id.or(status.installed_build.game_version)
        }))
    }
}

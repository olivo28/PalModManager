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

#[inline]
fn contains_ignore_case_ascii(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let n_bytes = needle.as_bytes();
    let h_bytes = haystack.as_bytes();
    if h_bytes.len() < n_bytes.len() {
        return false;
    }
    h_bytes.windows(n_bytes.len()).any(|w| {
        w.iter().zip(n_bytes.iter()).all(|(a, b)| a.to_ascii_lowercase() == b.to_ascii_lowercase())
    })
}

#[inline]
fn starts_with_ignore_case_ascii(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let n_bytes = needle.as_bytes();
    let h_bytes = haystack.as_bytes();
    if h_bytes.len() < n_bytes.len() {
        return false;
    }
    h_bytes[..n_bytes.len()].iter().zip(n_bytes.iter()).all(|(a, b)| a.to_ascii_lowercase() == b.to_ascii_lowercase())
}

enum MatchedItemRef<'a> {
    Struct(&'a str, &'a UsmapStruct),
    Enum(&'a str, &'a Vec<String>),
    FName(&'a str),
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

    let q = query.trim();
    let mut matched_refs: Vec<MatchedItemRef> = Vec::with_capacity(1024);

    // 1. Structs / Classes
    if filter_type == "all" || filter_type == "structs" {
        for (name, s) in &schema.structs {
            if q.is_empty() 
                || contains_ignore_case_ascii(name, q) 
                || s.super_type.as_ref().map_or(false, |st| contains_ignore_case_ascii(st, q)) 
            {
                matched_refs.push(MatchedItemRef::Struct(name.as_str(), s));
            }
        }
    }

    // 2. Enums
    if filter_type == "all" || filter_type == "enums" {
        for (name, vals) in &schema.enums {
            if q.is_empty() 
                || contains_ignore_case_ascii(name, q) 
                || vals.iter().any(|v| contains_ignore_case_ascii(v, q)) 
            {
                matched_refs.push(MatchedItemRef::Enum(name.as_str(), vals));
            }
        }
    }

    // 3. FNames
    if filter_type == "all" || filter_type == "names" {
        for name in &schema.names {
            if q.is_empty() || contains_ignore_case_ascii(name, q) {
                matched_refs.push(MatchedItemRef::FName(name.as_str()));
            }
        }
    }

    // Sort matching references: Exact match -> Starts with -> Alphabetical
    matched_refs.sort_by(|a_ref, b_ref| {
        let a_name = match a_ref {
            MatchedItemRef::Struct(n, _) => *n,
            MatchedItemRef::Enum(n, _) => *n,
            MatchedItemRef::FName(n) => *n,
        };
        let b_name = match b_ref {
            MatchedItemRef::Struct(n, _) => *n,
            MatchedItemRef::Enum(n, _) => *n,
            MatchedItemRef::FName(n) => *n,
        };

        if !q.is_empty() {
            let a_exact = a_name.eq_ignore_ascii_case(q);
            let b_exact = b_name.eq_ignore_ascii_case(q);
            if a_exact && !b_exact {
                return std::cmp::Ordering::Less;
            } else if !a_exact && b_exact {
                return std::cmp::Ordering::Greater;
            }

            let a_starts = starts_with_ignore_case_ascii(a_name, q);
            let b_starts = starts_with_ignore_case_ascii(b_name, q);
            if a_starts && !b_starts {
                return std::cmp::Ordering::Less;
            } else if !a_starts && b_starts {
                return std::cmp::Ordering::Greater;
            }
        }

        a_name.cmp(b_name)
    });

    let total_items = matched_refs.len();
    let start_idx = page.saturating_mul(page_size);
    let end_idx = (start_idx + page_size).min(total_items);

    let mut items = Vec::with_capacity(end_idx.saturating_sub(start_idx));
    if start_idx < total_items {
        for r in &matched_refs[start_idx..end_idx] {
            match r {
                MatchedItemRef::Struct(name, s) => {
                    let preview = if let Some(ref st) = s.super_type {
                        format!("Super: {} ({} properties)", st, s.properties.len())
                    } else {
                        format!("{} properties", s.properties.len())
                    };
                    items.push(UsmapSearchItem {
                        name: name.to_string(),
                        category: "struct".to_string(),
                        super_type: s.super_type.clone(),
                        property_count: s.properties.len(),
                        enum_values_count: 0,
                        preview,
                    });
                }
                MatchedItemRef::Enum(name, vals) => {
                    let preview = format!("{} values: {}", vals.len(), vals.iter().take(3).cloned().collect::<Vec<_>>().join(", "));
                    items.push(UsmapSearchItem {
                        name: name.to_string(),
                        category: "enum".to_string(),
                        super_type: None,
                        property_count: 0,
                        enum_values_count: vals.len(),
                        preview,
                    });
                }
                MatchedItemRef::FName(name) => {
                    items.push(UsmapSearchItem {
                        name: name.to_string(),
                        category: "name".to_string(),
                        super_type: None,
                        property_count: 0,
                        enum_values_count: 0,
                        preview: "FName Symbol".to_string(),
                    });
                }
            }
        }
    }

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

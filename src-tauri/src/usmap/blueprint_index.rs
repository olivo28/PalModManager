use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintEntry {
    pub name: String,
    pub class_name: String,
    pub package: String,
    pub full_path: String,
    pub super_class: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintIndex {
    pub total_blueprints: usize,
    pub blueprints: HashMap<String, BlueprintEntry>,
}

static ACTIVE_BLUEPRINT_CACHE: Mutex<Option<Arc<BlueprintIndex>>> = Mutex::new(None);

impl BlueprintIndex {
    pub fn find_blueprint(&self, name: &str) -> Option<&BlueprintEntry> {
        if let Some(entry) = self.blueprints.get(name) {
            return Some(entry);
        }
        let lower = name.to_ascii_lowercase();
        self.blueprints.values().find(|b| b.name.eq_ignore_ascii_case(&lower) || b.class_name.eq_ignore_ascii_case(&lower))
    }

    pub fn search_blueprints(&self, query: &str, limit: usize) -> Vec<&BlueprintEntry> {
        let q_lower = query.trim().to_ascii_lowercase();
        let mut results = Vec::new();
        let mut seen = HashSet::new();

        for entry in self.blueprints.values() {
            if (q_lower.is_empty() || entry.name.to_ascii_lowercase().contains(&q_lower) || entry.class_name.to_ascii_lowercase().contains(&q_lower))
                && seen.insert(&entry.class_name)
            {
                results.push(entry);
                if results.len() >= limit {
                    break;
                }
            }
        }
        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
    }
}

pub fn get_blueprints_dir(program_path: &str) -> PathBuf {
    let base = if !program_path.is_empty() {
        PathBuf::from(program_path)
    } else {
        #[cfg(target_os = "windows")]
        let p = std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("PalModManager");
        #[cfg(not(target_os = "windows"))]
        let p = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".local").join("share"))
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("PalModManager");
        p
    };
    base.join("resources").join("blueprints")
}

pub fn get_or_load_blueprint_index(program_path: &str) -> Option<Arc<BlueprintIndex>> {
    let mut cache = ACTIVE_BLUEPRINT_CACHE.lock().ok()?;
    if let Some(ref idx) = *cache {
        return Some(Arc::clone(idx));
    }

    let dir = get_blueprints_dir(program_path);
    let mut candidate_filename = "Palworld_Blueprints_24575825.json".to_string();

    let manifest_path = super::sync::find_bundled_resource("resources/blueprints/manifest.json")
        .or_else(|| {
            let p = dir.join("manifest.json");
            if p.exists() { Some(p) } else { None }
        });

    if let Some(mp) = manifest_path {
        if let Ok(m_str) = fs::read_to_string(&mp) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&m_str) {
                if let Some(list) = v.get("blueprints").and_then(|a| a.as_array()) {
                    if let Some(first) = list.first() {
                        if let Some(fname) = first.get("blueprints_filename").and_then(|s| s.as_str()) {
                            candidate_filename = fname.to_string();
                        }
                    }
                }
            }
        }
    }

    let rel_candidate = format!("resources/blueprints/{}", candidate_filename);
    let index_file_candidate = super::sync::find_bundled_resource(&rel_candidate)
        .or_else(|| {
            let p = dir.join(&candidate_filename);
            if p.exists() { Some(p) } else { None }
        })
        .or_else(|| {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let p = entry.path();
                    if p.is_file() {
                        if let Some(fname) = p.file_name().and_then(|n| n.to_str()) {
                            if fname.starts_with("Palworld_Blueprints_") && fname.ends_with(".json") {
                                return Some(p);
                            }
                        }
                    }
                }
            }
            None
        });

    if let Some(index_path) = index_file_candidate {
        match fs::read_to_string(&index_path) {
            Ok(content) => match serde_json::from_str::<BlueprintIndex>(&content) {
                Ok(index) => {
                    crate::logger::log(&format!(
                        "Loaded Palworld Blueprints Index: {} classes from {:?}",
                        index.total_blueprints, index_path
                    ));
                    let arc_idx = Arc::new(index);
                    *cache = Some(Arc::clone(&arc_idx));
                    return Some(arc_idx);
                }
                Err(e) => {
                    crate::logger::log(&format!("Failed to parse Blueprint index JSON from {:?}: {}", index_path, e));
                }
            },
            Err(e) => {
                crate::logger::log(&format!("Failed to read Blueprint index file at {:?}: {}", index_path, e));
            }
        }
    }

    None
}

pub fn invalidate_blueprint_cache() {
    if let Ok(mut cache) = ACTIVE_BLUEPRINT_CACHE.lock() {
        *cache = None;
    }
}

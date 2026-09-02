use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTableEntry {
    pub name: String,
    pub struct_name: String,
    pub package: String,
    pub count: usize,
    pub rows: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTableIndex {
    pub total_tables: usize,
    pub total_rows: usize,
    pub tables: HashMap<String, DataTableEntry>,
}

static ACTIVE_DATATABLE_CACHE: Mutex<Option<Arc<DataTableIndex>>> = Mutex::new(None);

impl DataTableIndex {
    /// Find a DataTable by exact name or case-insensitive match
    pub fn find_table(&self, name: &str) -> Option<&DataTableEntry> {
        if let Some(entry) = self.tables.get(name) {
            return Some(entry);
        }
        let lower = name.to_ascii_lowercase();
        self.tables.values().find(|t| t.name.eq_ignore_ascii_case(&lower))
    }

    /// Search DataTables matching query
    pub fn search_tables(&self, query: &str, limit: usize) -> Vec<&DataTableEntry> {
        let q_lower = query.trim().to_ascii_lowercase();
        let mut results = Vec::new();
        for entry in self.tables.values() {
            if q_lower.is_empty() || entry.name.to_ascii_lowercase().contains(&q_lower) {
                results.push(entry);
                if results.len() >= limit {
                    break;
                }
            }
        }
        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
    }

    /// Search rows inside a specific table, or across all tables if table_name is None
    /// Returns Vec<(row_key, table_name)>
    pub fn search_rows<'a>(&'a self, table_name: Option<&str>, query: &str, limit: usize) -> Vec<(&'a str, &'a str)> {
        let q_lower = query.trim().to_ascii_lowercase();
        let mut results = Vec::new();

        if let Some(tbl) = table_name.and_then(|t| self.find_table(t)) {
            for row in &tbl.rows {
                if q_lower.is_empty() || row.to_ascii_lowercase().contains(&q_lower) {
                    results.push((row.as_str(), tbl.name.as_str()));
                    if results.len() >= limit {
                        break;
                    }
                }
            }
        } else {
            for tbl in self.tables.values() {
                for row in &tbl.rows {
                    if q_lower.is_empty() || row.to_ascii_lowercase().contains(&q_lower) {
                        results.push((row.as_str(), tbl.name.as_str()));
                        if results.len() >= limit {
                            return results;
                        }
                    }
                }
            }
        }

        results
    }
}

pub fn get_datatables_dir(program_path: &str) -> PathBuf {
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
    base.join("resources").join("datatables")
}

pub fn get_or_load_datatable_index(program_path: &str) -> Option<Arc<DataTableIndex>> {
    let mut cache = ACTIVE_DATATABLE_CACHE.lock().ok()?;
    if let Some(ref idx) = *cache {
        return Some(Arc::clone(idx));
    }

    // 1. Try resolving canonical filename from manifest.json if present
    let dir = get_datatables_dir(program_path);
    let mut candidate_filename = "Palworld_DataTables_24575825.json".to_string();

    let manifest_path = super::sync::find_bundled_resource("resources/datatables/manifest.json")
        .or_else(|| {
            let p = dir.join("manifest.json");
            if p.exists() { Some(p) } else { None }
        });

    if let Some(mp) = manifest_path {
        if let Ok(m_str) = fs::read_to_string(&mp) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&m_str) {
                if let Some(list) = v.get("datatables").and_then(|a| a.as_array()) {
                    if let Some(first) = list.first() {
                        if let Some(fname) = first.get("datatables_filename").and_then(|s| s.as_str()) {
                            candidate_filename = fname.to_string();
                        }
                    }
                }
            }
        }
    }

    let rel_candidate = format!("resources/datatables/{}", candidate_filename);
    let index_file_candidate = super::sync::find_bundled_resource(&rel_candidate)
        .or_else(|| {
            let p = dir.join(&candidate_filename);
            if p.exists() { Some(p) } else { None }
        })
        .or_else(|| {
            // Find any Palworld_DataTables_*.json file in datatables directory
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let p = entry.path();
                    if p.is_file() {
                        if let Some(fname) = p.file_name().and_then(|n| n.to_str()) {
                            if fname.starts_with("Palworld_DataTables_") && fname.ends_with(".json") {
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
            Ok(content) => match serde_json::from_str::<DataTableIndex>(&content) {
                Ok(index) => {
                    crate::logger::log(&format!(
                        "Loaded Palworld DataTables Index: {} tables, {} rows from {:?}",
                        index.total_tables, index.total_rows, index_path
                    ));
                    let arc_idx = Arc::new(index);
                    *cache = Some(Arc::clone(&arc_idx));
                    return Some(arc_idx);
                }
                Err(e) => {
                    crate::logger::log(&format!("Failed to parse DataTable index JSON from {:?}: {}", index_path, e));
                }
            },
            Err(e) => {
                crate::logger::log(&format!("Failed to read DataTable index file at {:?}: {}", index_path, e));
            }
        }
    }

    None
}

pub fn invalidate_datatable_cache() {
    if let Ok(mut cache) = ACTIVE_DATATABLE_CACHE.lock() {
        *cache = None;
    }
}

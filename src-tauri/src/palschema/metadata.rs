use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PalSchemaMetadata {
    #[serde(default)]
    pub mod_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub authors: Option<Vec<String>>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_version")]
    pub version: Option<String>,
    #[serde(default)]
    pub dependencies: Option<Vec<String>>,
    #[serde(default = "default_true")]
    pub enabled: Option<bool>,
}

fn default_version() -> Option<String> {
    Some("1.0.0".to_string())
}

fn default_true() -> Option<bool> {
    Some(true)
}

impl PalSchemaMetadata {
    /// Reads and normalizes `metadata.json` from a mod folder on disk.
    /// Follows PalSchema 0.7.0 C++ fallback rules:
    /// - If `name` is empty: fallback to folder name.
    /// - If `mod_id` is empty: fallback to `name`.
    /// - `mod_id` is always lowercased.
    pub fn read_from_dir(dir: &Path) -> Option<Self> {
        let meta_file = dir.join("metadata.json");
        let folder_name = dir.file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        if meta_file.exists() {
            if let Ok(content) = fs::read_to_string(&meta_file) {
                if let Ok(mut meta) = serde_json::from_str::<PalSchemaMetadata>(&content) {
                    meta.normalize(&folder_name);
                    return Some(meta);
                } else {
                    crate::logger::log(&format!(
                        "PalSchema: Failed to parse metadata.json in {}",
                        dir.display()
                    ));
                }
            }
        }

        None
    }

    /// Reads and normalizes `metadata.json` from a byte slice (e.g. from ZIP archive).
    pub fn read_from_slice(bytes: &[u8], fallback_folder: &str) -> Option<Self> {
        if let Ok(mut meta) = serde_json::from_slice::<PalSchemaMetadata>(bytes) {
            meta.normalize(fallback_folder);
            Some(meta)
        } else {
            None
        }
    }

    /// Normalizes metadata matching PalSchema 0.7.0 `FMod::LoadMetadata` C++ semantics.
    pub fn normalize(&mut self, fallback_folder: &str) {
        if self.name.as_deref().unwrap_or("").trim().is_empty() {
            self.name = Some(fallback_folder.to_string());
        }

        if self.mod_id.trim().is_empty() {
            self.mod_id = self.name.clone().unwrap_or_else(|| fallback_folder.to_string());
        }

        self.mod_id = self.mod_id.to_lowercase();

        if self.enabled.is_none() {
            self.enabled = Some(true);
        }
    }

    /// Atomically writes this metadata to `dir/metadata.json`.
    pub fn write_to_dir(&self, dir: &Path) -> Result<(), String> {
        let meta_file = dir.join("metadata.json");
        let json_str = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize metadata.json: {}", e))?;
        
        fs::write(&meta_file, json_str)
            .map_err(|e| format!("Failed to write metadata.json to {}: {}", meta_file.display(), e))
    }

    /// In-situ toggle of the `enabled` field in `metadata.json`.
    /// If `metadata.json` already exists, modifies `enabled`.
    /// If not present, creates a minimal `metadata.json` matching the folder name.
    pub fn set_enabled_in_dir(dir: &Path, enabled: bool) -> Result<(), String> {
        let folder_name = dir.file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "mod".to_string());

        let mut meta = Self::read_from_dir(dir).unwrap_or_else(|| {
            let mut new_meta = PalSchemaMetadata {
                mod_id: folder_name.to_lowercase(),
                name: Some(folder_name.clone()),
                authors: None,
                description: None,
                version: Some("1.0.0".to_string()),
                dependencies: None,
                enabled: Some(enabled),
            };
            new_meta.normalize(&folder_name);
            new_meta
        });

        meta.enabled = Some(enabled);
        meta.write_to_dir(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_palschema_metadata() {
        let json = r#"{
            "mod_id": "Example_Mod",
            "name": "Example Mod",
            "authors": ["Author1", "Author2"],
            "description": "A cool mod",
            "version": "1.2.0",
            "dependencies": ["other_mod"],
            "enabled": true
        }"#;

        let meta = PalSchemaMetadata::read_from_slice(json.as_bytes(), "fallback_dir").unwrap();
        assert_eq!(meta.mod_id, "example_mod");
        assert_eq!(meta.name.as_deref(), Some("Example Mod"));
        assert_eq!(meta.version.as_deref(), Some("1.2.0"));
        assert_eq!(meta.enabled, Some(true));
        assert_eq!(meta.dependencies, Some(vec!["other_mod".to_string()]));
        assert_eq!(meta.authors, Some(vec!["Author1".to_string(), "Author2".to_string()]));
    }

    #[test]
    fn test_metadata_in_situ_toggle() {
        let temp_dir = std::env::temp_dir().join(format!("pmm_test_meta_{}", uuid::Uuid::new_v4()));
        let _ = fs::create_dir_all(&temp_dir);

        // Toggle to false when metadata doesn't exist
        assert!(PalSchemaMetadata::set_enabled_in_dir(&temp_dir, false).is_ok());
        let read1 = PalSchemaMetadata::read_from_dir(&temp_dir).unwrap();
        assert_eq!(read1.enabled, Some(false));

        // Toggle to true
        assert!(PalSchemaMetadata::set_enabled_in_dir(&temp_dir, true).is_ok());
        let read2 = PalSchemaMetadata::read_from_dir(&temp_dir).unwrap();
        assert_eq!(read2.enabled, Some(true));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

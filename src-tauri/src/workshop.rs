use crate::models::{PalModSettings, WorkshopMod, WorkshopInstallType};
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

const WORKSHOP_FRAMEWORK_IDS: &[u64] = &[3625223587, 3625280368];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SteamInstallManifest {
    #[serde(rename = "Files")]
    files: Vec<String>,
    #[serde(rename = "Dirs")]
    dirs: Vec<String>,
    #[serde(rename = "Backups")]
    backups: Vec<String>,
    #[serde(rename = "WorkshopId")]
    workshop_id: u64,
    #[serde(rename = "LastInstallTimeUtc")]
    last_install_time_utc: String,
    #[serde(rename = "LastWorkshopUpdateTimeUtc")]
    last_workshop_update_time_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkshopInfoJson {
    #[serde(rename = "ModName")]
    mod_name: String,
    #[serde(rename = "PackageName")]
    package_name: String,
    #[serde(rename = "Version")]
    version: String,
    #[serde(rename = "Author")]
    author: String,
    #[serde(rename = "Dependencies")]
    dependencies: Option<Vec<String>>,
    #[serde(rename = "InstallRule")]
    install_rule: Vec<InstallRuleJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstallRuleJson {
    #[serde(rename = "Type")]
    rule_type: String,
    #[serde(rename = "Targets")]
    targets: Vec<String>,
}

pub fn read_pal_mod_settings(game_path: &str) -> PalModSettings {
    let path = Path::new(game_path).join("Mods").join("PalModSettings.ini");

    let mut settings = PalModSettings::default();
    settings.config_version = "1.0".to_string();

    if let Ok(content) = fs::read_to_string(&path) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if let Some(pos) = line.find('=') {
                let key = line[..pos].trim();
                let val = line[pos + 1..].trim();
                match key.to_lowercase().as_str() {
                    "bglobalenablemod" => {
                        settings.global_enabled = val.to_lowercase() == "true";
                    }
                    "workshoprootdir" => {
                        settings.workshop_root = val.to_string();
                    }
                    "configversion" => {
                        settings.config_version = val.to_string();
                    }
                    "activemodlist" => {
                        if !val.is_empty() {
                            settings.active_mod_list.push(val.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    settings
}

pub fn write_pal_mod_settings(game_path: &str, settings: &PalModSettings) -> Result<(), String> {
    let ini_dir = Path::new(game_path).join("Mods");
    let _ = fs::create_dir_all(&ini_dir);
    let path = ini_dir.join("PalModSettings.ini");

    let mut lines = Vec::new();
    lines.push("[PalModSettings]".to_string());
    lines.push(format!("bGlobalEnableMod={}", if settings.global_enabled { "True" } else { "False" }));
    lines.push(format!("WorkshopRootDir={}", settings.workshop_root));
    lines.push(format!("ConfigVersion={}", settings.config_version));
    for active in &settings.active_mod_list {
        lines.push(format!("ActiveModList={}", active));
    }
    lines.push(String::new());

    let content = lines.join("\r\n");
    fs::write(&path, content).map_err(|e| format!("Failed to write PalModSettings.ini: {}", e))
}

pub fn scan_workshop_mods(game_path: &str) -> Vec<WorkshopMod> {
    let settings = read_pal_mod_settings(game_path);
    if settings.workshop_root.is_empty() {
        return Vec::new();
    }
    let workshop_dir = Path::new(&settings.workshop_root);
    if !workshop_dir.exists() {
        return Vec::new();
    }

    let mut mods = Vec::new();
    if let Ok(entries) = fs::read_dir(workshop_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let info_path = path.join("Info.json");
            if !info_path.exists() {
                continue;
            }
            let workshop_id: u64 = path.file_name()
                .and_then(|n| n.to_string_lossy().parse().ok())
                .unwrap_or(0);

            if let Ok(info_str) = fs::read_to_string(&info_path) {
                if let Ok(info) = serde_json::from_str::<WorkshopInfoJson>(&info_str) {
                    let is_active = settings.active_mod_list.contains(&info.package_name);
                    let is_framework = WORKSHOP_FRAMEWORK_IDS.contains(&workshop_id);
                    
                    let manifest_dir = Path::new(game_path).join("Mods").join("ManagedMods").join(&info.package_name);
                    let is_installed = manifest_dir.exists();
                    let manifest_file = manifest_dir.join("InstallManifest.json");
                    
                    let mut last_install = None;
                    let mut last_update = None;
                    if let Ok(manifest_str) = fs::read_to_string(&manifest_file) {
                        if let Ok(manifest) = serde_json::from_str::<SteamInstallManifest>(&manifest_str) {
                            last_install = Some(manifest.last_install_time_utc);
                            last_update = Some(manifest.last_workshop_update_time_utc);
                        }
                    }

                    let rule = info.install_rule.first();
                    let (install_type, install_target) = match rule {
                        Some(r) => {
                            let t = r.targets.first().cloned().unwrap_or_else(|| ".".to_string());
                            let it = match (r.rule_type.as_str(), t.as_str()) {
                                ("UE4SS", ".") => WorkshopInstallType::UE4SSFramework,
                                ("UE4SS", "./UE4SS/Mods") => WorkshopInstallType::UE4SSMod,
                                ("Lua", ".") => WorkshopInstallType::LuaMod,
                                ("PalSchema", "./PalSchema/") => WorkshopInstallType::PalSchemaMod,
                                (other, _) => WorkshopInstallType::Unknown(other.to_string()),
                            };
                            (it, t)
                        }
                        None => (WorkshopInstallType::Unknown("None".to_string()), ".".to_string()),
                    };

                    let thumb = path.join("thumbnail.png");
                    let thumbnail_path = if thumb.exists() {
                        Some(thumb.to_string_lossy().to_string())
                    } else {
                        None
                    };

                    mods.push(WorkshopMod {
                        workshop_id,
                        package_name: info.package_name,
                        mod_name: info.mod_name,
                        version: info.version,
                        author: info.author,
                        thumbnail_path,
                        dependencies: info.dependencies.unwrap_or_default(),
                        install_type,
                        install_target,
                        is_active,
                        is_installed,
                        is_framework,
                        last_install_time: last_install,
                        last_update_time: last_update,
                        has_pending_update: false,
                    });
                }
            }
        }
    }
    mods
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>, files: &mut Vec<String>, dirs: &mut Vec<String>, game_root: &Path) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    let relative_dir = pathdiff::diff_paths(&dst, game_root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    if !relative_dir.is_empty() {
        dirs.push(relative_dir);
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()), files, dirs, game_root)?;
        } else {
            let dst_file = dst.as_ref().join(entry.file_name());
            fs::copy(entry.path(), &dst_file)?;
            let relative_file = pathdiff::diff_paths(&dst_file, game_root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if !relative_file.is_empty() {
                files.push(relative_file);
            }
        }
    }
    Ok(())
}

pub fn activate_workshop_mod(game_path: &str, workshop_mod: &WorkshopMod) -> Result<(), String> {
    let settings = read_pal_mod_settings(game_path);
    if settings.workshop_root.is_empty() {
        return Err("Workshop root directory not configured".to_string());
    }
    let src_dir = Path::new(&settings.workshop_root).join(workshop_mod.workshop_id.to_string());
    if !src_dir.exists() {
        return Err(format!("Workshop files for {} not found on disk", workshop_mod.mod_name));
    }

    let mut installed_files = Vec::new();
    let mut installed_dirs = Vec::new();
    let game_root = Path::new(game_path);

    // Perform copy depending on type
    match workshop_mod.install_type {
        WorkshopInstallType::UE4SSMod => {
            let src_mod_dir = src_dir.join("UE4SS").join("Mods");
            let dest_mod_dir = game_root.join("Mods").join("NativeMods").join("UE4SS").join("Mods");
            if src_mod_dir.exists() {
                let _ = copy_dir_all(&src_mod_dir, &dest_mod_dir, &mut installed_files, &mut installed_dirs, game_root);
            }
        }
        WorkshopInstallType::PalSchemaMod => {
            let src_schema_dir = src_dir.join("PalSchema");
            let dest_schema_dir = game_root.join("Mods").join("NativeMods").join("UE4SS").join("Mods").join("PalSchema").join("mods").join(&workshop_mod.package_name);
            if src_schema_dir.exists() {
                let _ = copy_dir_all(&src_schema_dir, &dest_schema_dir, &mut installed_files, &mut installed_dirs, game_root);
            }
        }
        WorkshopInstallType::LuaMod => {
            let dest_mod_dir = game_root.join("Mods").join("NativeMods").join("UE4SS").join("Mods").join(&workshop_mod.package_name);
            let _ = copy_dir_all(&src_dir, &dest_mod_dir, &mut installed_files, &mut installed_dirs, game_root);
            
            // Add entry to mods.txt
            let mods_txt = game_root.join("Mods").join("NativeMods").join("UE4SS").join("Mods").join("mods.txt");
            if mods_txt.exists() {
                if let Ok(content) = fs::read_to_string(&mods_txt) {
                    if !content.contains(&workshop_mod.package_name) {
                        let new_content = format!("{}{} : 1\r\n", content, workshop_mod.package_name);
                        let _ = fs::write(&mods_txt, new_content);
                    }
                }
            }
        }
        _ => {}
    }

    // Write metadata
    let managed_dir = game_root.join("Mods").join("ManagedMods").join(&workshop_mod.package_name);
    let _ = fs::create_dir_all(&managed_dir);
    
    let info_src = src_dir.join("Info.json");
    let info_dst = managed_dir.join("Info.json");
    if info_src.exists() {
        let _ = fs::copy(&info_src, &info_dst);
        let rel = pathdiff::diff_paths(&info_dst, game_root).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
        if !rel.is_empty() {
            installed_files.push(rel);
        }
    }
    
    installed_dirs.push(pathdiff::diff_paths(&managed_dir, game_root).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default());

    let now = chrono::Utc::now().to_rfc3339();
    let manifest = SteamInstallManifest {
        files: installed_files,
        dirs: installed_dirs,
        backups: Vec::new(),
        workshop_id: workshop_mod.workshop_id,
        last_install_time_utc: now.clone(),
        last_workshop_update_time_utc: now,
    };

    let manifest_file = managed_dir.join("InstallManifest.json");
    if let Ok(manifest_str) = serde_json::to_string_pretty(&manifest) {
        let _ = fs::write(&manifest_file, manifest_str);
    }

    // Update PalModSettings.ini
    let mut updated_settings = read_pal_mod_settings(game_path);
    if !updated_settings.active_mod_list.contains(&workshop_mod.package_name) {
        updated_settings.active_mod_list.push(workshop_mod.package_name.clone());
        let _ = write_pal_mod_settings(game_path, &updated_settings);
    }

    Ok(())
}

pub fn deactivate_workshop_mod(game_path: &str, workshop_mod: &WorkshopMod) -> Result<(), String> {
    let game_root = Path::new(game_path);
    let managed_dir = game_root.join("Mods").join("ManagedMods").join(&workshop_mod.package_name);
    let manifest_file = managed_dir.join("InstallManifest.json");

    if manifest_file.exists() {
        if let Ok(manifest_str) = fs::read_to_string(&manifest_file) {
            if let Ok(manifest) = serde_json::from_str::<SteamInstallManifest>(&manifest_str) {
                // Delete files
                for relative_file in &manifest.files {
                    let full_path = game_root.join(relative_file);
                    if full_path.exists() {
                        let _ = fs::remove_file(&full_path);
                    }
                }
                // Delete directories in reverse order
                let mut sorted_dirs = manifest.dirs.clone();
                sorted_dirs.sort_by_key(|b| std::cmp::Reverse(b.len()));
                for relative_dir in &sorted_dirs {
                    let full_path = game_root.join(relative_dir);
                    if full_path.exists() && fs::read_dir(&full_path).map(|mut d| d.next().is_none()).unwrap_or(false) {
                        let _ = fs::remove_dir(&full_path);
                    }
                }
            }
        }
    }

    // Clean leftovers
    if managed_dir.exists() {
        let _ = fs::remove_dir_all(&managed_dir);
    }

    // Remove from mods.txt if LuaMod
    if workshop_mod.install_type == WorkshopInstallType::LuaMod {
        let mods_txt = game_root.join("Mods").join("NativeMods").join("UE4SS").join("Mods").join("mods.txt");
        if mods_txt.exists() {
            if let Ok(content) = fs::read_to_string(&mods_txt) {
                let lines: Vec<String> = content.lines()
                    .filter(|l| !l.trim().starts_with(&workshop_mod.package_name))
                    .map(|l| l.to_string())
                    .collect();
                let _ = fs::write(&mods_txt, lines.join("\r\n") + "\r\n");
            }
        }
    }

    // Remove from PalModSettings.ini
    let mut updated_settings = read_pal_mod_settings(game_path);
    if let Some(pos) = updated_settings.active_mod_list.iter().position(|x| x == &workshop_mod.package_name) {
        updated_settings.active_mod_list.remove(pos);
        let _ = write_pal_mod_settings(game_path, &updated_settings);
    }

    Ok(())
}

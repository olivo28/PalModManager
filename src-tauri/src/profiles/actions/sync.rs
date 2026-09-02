use std::fs;
use std::path::Path;
use crate::models::{ModInfo, ModType};
use super::folder_name::get_mod_folder_name;

pub fn clean_mods_txt_native_only(mods_txt: &Path) -> Result<(), String> {
    if !mods_txt.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(mods_txt).unwrap_or_default();
    let mut native_lines = Vec::new();
    let mut bottom_lines = Vec::new();
    let mut in_bottom_keybinds = false;

    for line in content.lines() {
        let line_clean = line.trim();
        if line_clean.eq_ignore_ascii_case("; Built-in keybinds") 
            || line_clean.to_lowercase().starts_with("; built-in keybinds")
            || (line_clean.to_lowercase().starts_with("keybinds") && line_clean.contains(':'))
        {
            in_bottom_keybinds = true;
        }

        if in_bottom_keybinds {
            if !line_clean.is_empty() {
                bottom_lines.push(line_clean.to_string());
            }
            continue;
        }

        let name = if let Some(pos) = line_clean.find(':') {
            line_clean[..pos].trim()
        } else {
            line_clean
        };

        let is_native_tool = [
            "CheatManagerEnablerMod",
            "ConsoleCommandsMod",
            "ConsoleEnablerMod",
            "SplitScreenMod",
            "LineTraceMod",
            "BPML_GenericFunctions",
            "BPModLoaderMod",
        ].iter().any(|&n| n.eq_ignore_ascii_case(name));

        if is_native_tool {
            native_lines.push(line.to_string());
        }
    }

    if native_lines.is_empty() {
        native_lines = vec![
            "CheatManagerEnablerMod : 0".to_string(),
            "ConsoleCommandsMod : 0".to_string(),
            "ConsoleEnablerMod : 0".to_string(),
            "SplitScreenMod : 0".to_string(),
            "LineTraceMod : 0".to_string(),
            "BPML_GenericFunctions : 1".to_string(),
            "BPModLoaderMod : 1".to_string(),
        ];
    }

    if bottom_lines.is_empty() {
        bottom_lines = vec![
            "; Built-in keybinds, do not move up!".to_string(),
            "Keybinds : 1".to_string(),
        ];
    }

    let mut output_lines = Vec::new();
    for l in native_lines {
        output_lines.push(l);
    }
    output_lines.push("".to_string());
    for bl in bottom_lines {
        output_lines.push(bl);
    }

    fs::write(mods_txt, output_lines.join("\r\n") + "\r\n").map_err(|e| e.to_string())?;
    Ok(())
}

pub fn sync_mods_txt_sections(
    mods_txt: &Path,
    profile: &crate::models::Profile,
    mods: &[ModInfo],
) -> Result<(), String> {
    let content = fs::read_to_string(mods_txt).unwrap_or_default();
    
    // 1. Identify native UE4SS mods and bottom keybinds from existing content
    let mut native_lines = Vec::new();
    let mut bottom_lines = Vec::new();
    let mut in_bottom_keybinds = false;

    for line in content.lines() {
        let line_clean = line.trim();
        if line_clean.eq_ignore_ascii_case("; Built-in keybinds") 
            || line_clean.to_lowercase().starts_with("; built-in keybinds")
            || (line_clean.to_lowercase().starts_with("keybinds") && line_clean.contains(':'))
        {
            in_bottom_keybinds = true;
        }

        if in_bottom_keybinds {
            bottom_lines.push(line.to_string());
            continue;
        }

        // Check if this is a native UE4SS tool line at the top
        let name = if let Some(pos) = line_clean.find(':') {
            line_clean[..pos].trim()
        } else {
            line_clean
        };

        let is_native_tool = [
            "CheatManagerEnablerMod",
            "ConsoleCommandsMod",
            "ConsoleEnablerMod",
            "SplitScreenMod",
            "LineTraceMod",
            "BPML_GenericFunctions",
            "BPModLoaderMod",
        ].iter().any(|&n| n.eq_ignore_ascii_case(name));

        if is_native_tool {
            native_lines.push(line.to_string());
        }
    }

    if native_lines.is_empty() {
        native_lines = vec![
            "CheatManagerEnablerMod : 0".to_string(),
            "ConsoleCommandsMod : 0".to_string(),
            "ConsoleEnablerMod : 0".to_string(),
            "SplitScreenMod : 0".to_string(),
            "LineTraceMod : 0".to_string(),
            "BPML_GenericFunctions : 1".to_string(),
            "BPModLoaderMod : 1".to_string(),
        ];
    }

    // 2. Filter UE4SS/Hybrid mods installed in this profile
    let mut installed_ue4ss_mods: Vec<&ModInfo> = Vec::new();
    for m in mods {
        if (m.mod_type == ModType::Ue4ss || m.mod_type == ModType::Hybrid)
            && m.nexus_author.as_deref() != Some("UE4SS Native Mod")
        {
            if profile.installed_mod_ids.iter().any(|entry| crate::profiles::mod_matches_profile_entry(m, entry)) {
                installed_ue4ss_mods.push(m);
            }
        }
    }

    // 3. Build output lines starting with top native tools
    let mut output_lines = Vec::new();
    for l in native_lines {
        output_lines.push(l);
    }

    let mut assigned_mod_ids = std::collections::HashSet::new();

    // 4. For each virtual folder in profile.mod_folders:
    for folder in &profile.mod_folders {
        let mut active_mod_lines = Vec::new();
        let mut disabled_mod_lines = Vec::new();

        for fid in &folder.mod_ids {
            if let Some(m) = installed_ue4ss_mods.iter().find(|m| crate::profiles::mod_matches_profile_entry(m, fid)) {
                let folder_name = get_mod_folder_name(m);
                let is_enabled = m.enabled && (profile.enabled_mod_ids.is_empty() || profile.enabled_mod_ids.iter().any(|id| crate::profiles::mod_matches_profile_entry(m, id)));
                if is_enabled {
                    active_mod_lines.push(format!("{} : 1", folder_name));
                } else {
                    disabled_mod_lines.push(format!("{} : 0", folder_name));
                }
                assigned_mod_ids.insert(m.id.clone());
            }
        }

        if !active_mod_lines.is_empty() || !disabled_mod_lines.is_empty() {
            output_lines.push("".to_string());
            output_lines.push(format!("; -----{}-----", folder.name));
            for aml in active_mod_lines {
                output_lines.push(aml);
            }
            output_lines.push("; -----Disabled Mods-----".to_string());
            for dml in disabled_mod_lines {
                output_lines.push(dml);
            }
        }
    }

    // 5. Any ungrouped UE4SS mods in this profile
    let mut ungrouped_active = Vec::new();
    let mut ungrouped_disabled = Vec::new();
    for m in &installed_ue4ss_mods {
        if !assigned_mod_ids.contains(&m.id) {
            let folder_name = get_mod_folder_name(m);
            let is_enabled = m.enabled && (profile.enabled_mod_ids.is_empty() || profile.enabled_mod_ids.iter().any(|id| crate::profiles::mod_matches_profile_entry(m, id)));
            if is_enabled {
                ungrouped_active.push(format!("{} : 1", folder_name));
            } else {
                ungrouped_disabled.push(format!("{} : 0", folder_name));
            }
        }
    }

    if !ungrouped_active.is_empty() || !ungrouped_disabled.is_empty() {
        output_lines.push("".to_string());
        for aml in ungrouped_active {
            output_lines.push(aml);
        }
        if !ungrouped_disabled.is_empty() {
            output_lines.push("; -----Disabled Mods-----".to_string());
            for dml in ungrouped_disabled {
                output_lines.push(dml);
            }
        }
    }

    // 6. Append bottom keybinds (always ensure a blank line before it!)
    let mut clean_bottom = Vec::new();
    for bl in bottom_lines {
        let trimmed = bl.trim();
        if !trimmed.is_empty() {
            clean_bottom.push(trimmed.to_string());
        }
    }

    if clean_bottom.is_empty() {
        clean_bottom = vec![
            "; Built-in keybinds, do not move up!".to_string(),
            "Keybinds : 1".to_string(),
        ];
    }

    output_lines.push("".to_string());
    for bl in clean_bottom {
        output_lines.push(bl);
    }

    fs::write(mods_txt, output_lines.join("\r\n") + "\r\n").map_err(|e| e.to_string())?;
    Ok(())
}

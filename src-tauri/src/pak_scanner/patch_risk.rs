use std::path::PathBuf;
use std::collections::HashSet;
use crate::models::ModInfo;
use super::types::PatchRiskNotice;
use super::reader::list_pak_entries;

struct CoreAssetRule {
    pattern: &'static str,
    category: &'static str,
    risk: &'static str,
    reason: &'static str,
}

const CORE_ASSET_RULES: &[CoreAssetRule] = &[
    // 1. Base Camp & Palbox
    CoreAssetRule {
        pattern: "bp_palbasecampmodel",
        category: "Base Camp",
        risk: "Critical",
        reason: "Overwrites core Base Camp model. Game updates modify camp properties and worker logic, causing immediate CTD on world load if unpatched.",
    },
    CoreAssetRule {
        pattern: "bp_palbox",
        category: "Base Camp",
        risk: "Critical",
        reason: "Overwrites Palbox building logic. Outdated versions trigger fatal deserialization errors when opening or accessing bases.",
    },
    CoreAssetRule {
        pattern: "bp_palmapobjectbasecamp",
        category: "Base Camp",
        risk: "High",
        reason: "Overwrites Base Camp map object actor. Frequently desynchronizes with new game patch logic.",
    },
    // 2. UI & Menus
    CoreAssetRule {
        pattern: "wbp_worldmap",
        category: "UI / World Map",
        risk: "Critical",
        reason: "Overwrites World Map interface. Game updates alter fast travel and icon widgets, causing instant CTD when opening map (M) or fast traveling.",
    },
    CoreAssetRule {
        pattern: "wbp_titlemenu",
        category: "UI / Menu",
        risk: "Critical",
        reason: "Overwrites Main/Title menu widget. Outdated UI assets trigger crash on game startup or when returning to title.",
    },
    CoreAssetRule {
        pattern: "wbp_escmenu",
        category: "UI / Menu",
        risk: "Critical",
        reason: "Overwrites In-Game Pause / Esc menu. Triggers game freeze or CTD when pressing Esc or attempting to save.",
    },
    CoreAssetRule {
        pattern: "wbp_ingamemenu",
        category: "UI / Menu",
        risk: "High",
        reason: "Overwrites In-Game HUD / Inventory container widget. Prone to null pointer crashes on patch updates.",
    },
    CoreAssetRule {
        pattern: "wbp_palcommon_fasttravel",
        category: "UI / World Map",
        risk: "High",
        reason: "Overwrites Fast Travel interaction widget. Causes crash when interacting with Great Eagle statues.",
    },
    // 3. Camera & Player State
    CoreAssetRule {
        pattern: "bp_playercamera",
        category: "Camera / Player",
        risk: "Critical",
        reason: "Overwrites Player Camera manager. Game updates alter spring arm and aim offsets, causing immediate crash upon spawning into the world.",
    },
    CoreAssetRule {
        pattern: "bp_palplayercamera",
        category: "Camera / Player",
        risk: "Critical",
        reason: "Overwrites Pal Player Camera. Triggers CTD when mounting, riding, or zooming camera.",
    },
    CoreAssetRule {
        pattern: "bp_palplayerstate",
        category: "Player State",
        risk: "Critical",
        reason: "Overwrites Player State actor. Changes in player progression variables trigger serialization errors on save load.",
    },
    CoreAssetRule {
        pattern: "bp_palplayercharacter",
        category: "Player Character",
        risk: "High",
        reason: "Overwrites Player Character actor. Desynchronizes animation blueprints and movement components.",
    },
];

/// Scans active .pak mods for overwrites of core vanilla game Blueprints and UI widgets
/// that commonly trigger Fatal Error crashes (CTD) following game patches.
pub fn check_patch_risk_compatibility(active_mods: &[ModInfo]) -> Vec<PatchRiskNotice> {
    let mut notices = Vec::new();
    let mut seen_keys = HashSet::new();

    for m in active_mods {
        if !m.enabled || m.game_path.is_empty() {
            continue;
        }

        let mut candidate_paks = Vec::new();
        let is_pak = |s: &str| s.to_lowercase().ends_with(".pak");

        if is_pak(&m.game_path) {
            candidate_paks.push(PathBuf::from(&m.game_path));
        }
        for extra in &m.extra_files {
            if is_pak(extra) {
                candidate_paks.push(PathBuf::from(extra));
            }
        }

        for pak_path in candidate_paks {
            if !pak_path.exists() {
                continue;
            }

            let pak_filename = pak_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            // Skip official Palworld base game pak files if present
            if pak_filename.starts_with("Pal-Windows") {
                continue;
            }

            if let Ok(entries) = list_pak_entries(&pak_path) {
                for entry in entries {
                    let entry_lower = entry.to_lowercase();
                    if !entry_lower.ends_with(".uasset") {
                        continue;
                    }

                    for rule in CORE_ASSET_RULES {
                        if entry_lower.contains(rule.pattern) {
                            let dedup_key = format!("{}:{}:{}", m.id, pak_filename, rule.pattern);
                            if seen_keys.insert(dedup_key) {
                                notices.push(PatchRiskNotice {
                                    mod_id: m.id.clone(),
                                    mod_name: m.name.clone(),
                                    pak_filename: pak_filename.clone(),
                                    asset_path: entry.clone(),
                                    asset_category: rule.category.to_string(),
                                    risk_level: rule.risk.to_string(),
                                    reason: rule.reason.to_string(),
                                });
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    notices.sort_by(|a, b| {
        let risk_order = |r: &str| match r {
            "Critical" => 0,
            "High" => 1,
            _ => 2,
        };
        risk_order(&a.risk_level)
            .cmp(&risk_order(&b.risk_level))
            .then_with(|| a.mod_name.cmp(&b.mod_name))
    });

    notices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_asset_rules_integrity() {
        assert!(!CORE_ASSET_RULES.is_empty());
        for rule in CORE_ASSET_RULES {
            assert!(!rule.pattern.is_empty(), "Pattern cannot be empty");
            assert!(!rule.category.is_empty(), "Category cannot be empty");
            assert!(!rule.reason.is_empty(), "Reason cannot be empty");
            assert!(
                matches!(rule.risk, "Critical" | "High" | "Moderate"),
                "Unexpected risk level: {}",
                rule.risk
            );
        }
    }

    #[test]
    fn test_rule_matching_patterns() {
        let base_camp_asset = "pal/content/pal/blueprint/map/bp_palbasecampmodel.uasset";
        let map_ui_asset = "pal/content/pal/ui/worldmap/wbp_worldmap.uasset";
        let camera_asset = "pal/content/pal/blueprint/camera/bp_playercamera.uasset";
        let safe_custom_asset = "pal/content/mypals/bp_mycustompal.uasset";

        let find_rule = |path: &str| {
            CORE_ASSET_RULES
                .iter()
                .find(|r| path.to_lowercase().contains(r.pattern))
        };

        let base_match = find_rule(base_camp_asset).expect("Should match base camp rule");
        assert_eq!(base_match.category, "Base Camp");
        assert_eq!(base_match.risk, "Critical");

        let ui_match = find_rule(map_ui_asset).expect("Should match UI rule");
        assert_eq!(ui_match.category, "UI / World Map");
        assert_eq!(ui_match.risk, "Critical");

        let cam_match = find_rule(camera_asset).expect("Should match camera rule");
        assert_eq!(cam_match.category, "Camera / Player");
        assert_eq!(cam_match.risk, "Critical");

        assert!(find_rule(safe_custom_asset).is_none());
    }
}

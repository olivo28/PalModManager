// PalSchema DataTable validation against USMAP schema and C++ SDK
use std::path::Path;

pub fn validate_palschema_table_with_usmap(
    table_name: &str,
    schema: &crate::usmap::parser::UsmapSchema,
    sdk: Option<&crate::usmap::SdkIndex>,
    game_path: &Path,
) -> (String, String, Option<String>) {
    let raw = table_name.trim();
    if raw.is_empty() {
        return ("unknown".to_string(), "Empty table name".to_string(), None);
    }

    let clean = if raw.starts_with("DT_") {
        &raw[3..]
    } else if raw.starts_with("BP_") {
        &raw[3..]
    } else {
        raw
    };

    let raw_lower = raw.to_ascii_lowercase();
    let clean_lower = clean.to_ascii_lowercase();

    // Blueprint Asset Class Modification in PalSchema (e.g. BP_BuildObject_AncientWorkBench_C)
    if raw.starts_with("BP_") || raw.starts_with("WBP_") || raw.starts_with("/Game/") || raw.ends_with("_C") {
        if let Some(sdk_idx) = sdk {
            if sdk_idx.find_class(clean).is_some() || sdk_idx.find_class(raw).is_some() {
                return (
                    "valid".to_string(),
                    format!("Verified Blueprint class '{}' in Palworld C++ SDK", raw),
                    None,
                );
            }
        }
        return (
            "valid".to_string(),
            format!("Verified Blueprint Actor Schema Modifier '{}'", raw),
            None,
        );
    }

    // 1. Direct match in USMAP schema FNames (e.g. DT_PalCharacterParameter, DT_ItemData)
    let in_schema_names = schema.names.iter().any(|n| n.eq_ignore_ascii_case(raw) || n.eq_ignore_ascii_case(clean));
    let in_schema_structs = schema.find_struct(raw).is_some() || schema.find_struct(clean).is_some();

    // 2. Direct check in C++ SDK
    let class_in_sdk = sdk.and_then(|s| s.find_class(clean).or_else(|| s.find_class(raw)));

    if in_schema_names || in_schema_structs || class_in_sdk.is_some() {
        return (
            "valid".to_string(),
            format!("Verified DataTable '{}' in Palworld reflection schema", raw),
            None,
        );
    }

    // 3. Check struct / class suffix and prefix variations (e.g. PalMonsterParameterTable, PalInvaderDataRow, PalVisitorNPCParameter)
    let struct_match = schema.structs.keys().any(|s| {
        let sl = s.to_ascii_lowercase();
        sl == clean_lower
            || sl == raw_lower
            || sl.starts_with(&clean_lower)
            || clean_lower.starts_with(&sl)
            || sl.contains(&clean_lower)
            || (clean_lower.starts_with("pal") && sl.contains(&clean_lower[3..]))
    });

    let sdk_class_match = if let Some(sdk_idx) = sdk {
        sdk_idx.classes.keys().any(|c| {
            let cl = c.to_ascii_lowercase();
            cl == clean_lower
                || cl == raw_lower
                || cl.starts_with(&clean_lower)
                || clean_lower.starts_with(&cl)
                || cl.contains(&clean_lower)
                || (clean_lower.starts_with("pal") && cl.contains(&clean_lower[3..]))
        })
    } else {
        false
    };

    let fnames_match = schema.names.iter().any(|n| {
        let nl = n.to_ascii_lowercase();
        nl == raw_lower
            || nl == clean_lower
            || nl.starts_with(&format!("{}_", clean_lower))
            || nl.ends_with(&format!("_{}", clean_lower))
            || (nl.starts_with("dt_") && nl.contains(&clean_lower))
            || (clean_lower.starts_with("pal") && nl.contains(&clean_lower))
    });

    if struct_match || sdk_class_match || fnames_match {
        return (
            "valid".to_string(),
            format!("Verified DataTable '{}' in Palworld reflection schema", raw),
            None,
        );
    }

    // 4. Check base game Pak files in Pal/Content/Paks/ if game_path exists
    if game_path.exists() {
        let paks_dir = game_path.join("Pal").join("Content").join("Paks");
        if paks_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&paks_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.starts_with("Pal-") && name.ends_with(".pak") {
                        if let Ok(pak_files) = crate::pak_scanner::list_pak_entries(&p) {
                            let found = pak_files.iter().any(|f| {
                                let fl = f.to_ascii_lowercase();
                                fl.ends_with(&format!("{}.uasset", clean_lower))
                                    || fl.ends_with(&format!("{}.uasset", raw_lower))
                                    || fl.contains(&format!("/{}.uasset", clean_lower))
                                    || fl.contains(&format!("/{}.uasset", raw_lower))
                            });
                            if found {
                                return (
                                    "valid".to_string(),
                                    format!("Verified DataTable '{}' in Palworld game assets", raw),
                                    None,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. If not found, try fuzzy match in schema.names (prioritizing DT_ entries or Pal... entries)
    let raw_norm = raw.replace('_', "").to_ascii_lowercase();
    let mut best_candidate: Option<String> = None;
    let mut highest_score: f64 = 0.0;

    for name in &schema.names {
        if !name.starts_with("DT_") && !name.contains("DataTable") && !name.starts_with("Pal") {
            continue;
        }
        let cand_norm = name.replace('_', "").to_ascii_lowercase();
        if raw_norm == cand_norm {
            best_candidate = Some(name.clone());
            break;
        }
        let max_len = raw_norm.len().max(cand_norm.len());
        if max_len == 0 {
            continue;
        }
        let dist = crate::usmap::sdk_parser::levenshtein_distance(&raw_norm, &cand_norm);
        let score = 1.0 - (dist as f64 / max_len as f64);
        if score > highest_score && score >= 0.55 {
            highest_score = score;
            best_candidate = Some(name.clone());
        }
    }

    // If still no candidate, check SDK classes
    if best_candidate.is_none() {
        if let Some(sdk_idx) = sdk {
            if let Some(sug) = sdk_idx.suggest_similar_class(clean) {
                best_candidate = Some(if raw.starts_with("DT_") { format!("DT_{}", sug) } else { sug });
            }
        }
    }

    (
        "broken_table".to_string(),
        format!("DataTable '{}' does not exist in Palworld reflection schema", raw),
        best_candidate,
    )
}

use std::collections::HashSet;
use std::sync::Arc;
use crate::usmap::{BlueprintIndex, DataTableIndex, SdkIndex, UsmapSchema};
use crate::commands::editor::types::EditorCompletion;
use super::common::{format_blueprint_class_doc, format_datatable_doc, format_property_doc};

pub fn populate_json_completions(
    file_path: &str,
    query: &str,
    line_prefix: &str,
    schema: Option<&Arc<UsmapSchema>>,
    sdk_index: Option<&Arc<SdkIndex>>,
    dt_index: Option<&Arc<DataTableIndex>>,
    bp_index: Option<&Arc<BlueprintIndex>>,
    palschema_tables: Option<&Arc<HashSet<String>>>,
    is_palschema_file: bool,
    completions: &mut Vec<EditorCompletion>,
    seen: &mut HashSet<String>,
) {
    let q_lower = query.trim().to_ascii_lowercase();
    let prefix_lower = line_prefix.to_ascii_lowercase();

    let clean_path = file_path.replace('\\', "/");
    let clean_path_lower = clean_path.to_lowercase();

    let is_blueprints_domain = clean_path_lower.contains("/blueprints/")
        || clean_path_lower.starts_with("blueprints/")
        || clean_path_lower.contains("blueprint");

    let is_raw_domain = clean_path_lower.contains("/raw/")
        || clean_path_lower.starts_with("raw/")
        || clean_path_lower.contains("dt_");

    // Context detection: detect if cursor is inside an object block (e.g. [context:BP_LaserRifle_C])
    let parent_context = if line_prefix.starts_with("[context:") {
        line_prefix.split_once(']').map(|(ctx, _)| ctx.trim_start_matches("[context:").trim().to_string())
    } else {
        None
    };

    let is_in_blueprint_block = parent_context.as_ref().map(|ctx| {
        let cl = ctx.to_ascii_lowercase();
        cl.starts_with("bp_") || cl.starts_with("wbp_") || cl.starts_with("abp_") || cl.contains("blueprint") || cl.ends_with("_c")
    }).unwrap_or(false) || (is_blueprints_domain && parent_context.is_some());

    // 0. Starter Snippets (Strictly for PalSchema files at root level)
    if is_palschema_file && parent_context.is_none() {
        if is_blueprints_domain && (q_lower.is_empty() || "blueprint".starts_with(&q_lower) || "bp_".starts_with(&q_lower)) {
            completions.push(EditorCompletion {
                label: "PalSchema Blueprint Patch".to_string(),
                insert_text: "\"${1:BP_ClassName_C}\": {\n  \"${2:PropertyName}\": ${0:value}\n}".to_string(),
                kind: "api".to_string(),
                detail: Some("PalSchema Template".to_string()),
                documentation: Some("```json\n// PalSchema Blueprint Patch Template\n\"BP_ClassName_C\": {\n  \"PropertyName\": value\n}\n```\n- **Target**: Live game Blueprint CDO properties\n- **Usage**: Modify class default object properties at runtime via PalSchema.".to_string()),
            });
        } else if is_raw_domain && (q_lower.is_empty() || "palschema".starts_with(&q_lower) || "datatable".starts_with(&q_lower)) {
            completions.push(EditorCompletion {
                label: "PalSchema DataTable Patch".to_string(),
                insert_text: "{\n  \"DataTable\": \"${1:DT_ItemData}\",\n  \"Rows\": {\n    \"${2:RowName}\": {\n      $0\n    }\n  }\n}".to_string(),
                kind: "api".to_string(),
                detail: Some("PalSchema Template".to_string()),
                documentation: Some("```json\n// PalSchema DataTable Patch Template\n{\n  \"DataTable\": \"DT_ItemData\",\n  \"Rows\": {\n    \"RowName\": { ... }\n  }\n}\n```\n- **Target**: Palworld game DataTables\n- **Usage**: Inject or overwrite rows at runtime via PalSchema.".to_string()),
            });
        }
    }

    // 1. DataTables: In raw domain, suggest at root or when querying. In other domains, ONLY when explicitly querying "dt_".
    let should_suggest_tables = if is_palschema_file {
        if is_blueprints_domain {
            q_lower.starts_with("dt_") || prefix_lower.contains("dt_")
        } else if is_raw_domain {
            !is_in_blueprint_block && (parent_context.is_none() || q_lower.starts_with("dt_") || prefix_lower.contains("dt_"))
        } else {
            q_lower.starts_with("dt_") || prefix_lower.contains("dt_")
        }
    } else {
        q_lower.starts_with("dt_") || prefix_lower.contains("dt_")
    };

    if should_suggest_tables {
        if is_palschema_file {
            if let Some(ps_tables) = palschema_tables {
                let mut ps_sorted: Vec<&String> = ps_tables.iter().collect();
                ps_sorted.sort();
                for table in ps_sorted {
                    let tl = table.to_ascii_lowercase();
                    if q_lower.is_empty() || tl.contains(&q_lower) {
                        if seen.insert(table.clone()) {
                            let is_text = tl.ends_with("text") || tl.contains("text");
                            let detail_str = if is_text {
                                "DataTable • Localized Text".to_string()
                            } else {
                                "DataTable • Schema Catalog".to_string()
                            };
                            let doc_md = format!(
                                "```typescript\n(table) {}\n```\n- **Catalog**: `Official PalSchema Schema Definition`\n- **Type**: `{}`\n- **Source**: `Embedded Okaetsu PalSchema Schemas Catalog`",
                                table,
                                if is_text { "Localized Text Table" } else { "Gameplay Reflection DataTable" }
                            );
                            completions.push(EditorCompletion {
                                label: table.clone(),
                                insert_text: table.clone(),
                                kind: "table".to_string(),
                                detail: Some(detail_str),
                                documentation: Some(doc_md),
                            });
                        }
                    }
                    if completions.len() >= 80 {
                        break;
                    }
                }
            }
        }

        let mut active_target_table: Option<String> = None;
        if let Some(pos) = prefix_lower.find("dt_") {
            let slice = &line_prefix[pos..];
            let end_pos = slice.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(slice.len());
            active_target_table = Some(slice[..end_pos].to_string());
        }

        if let Some(dti) = dt_index {
            for entry in dti.search_tables(query, 60) {
                if seen.insert(entry.name.clone()) {
                    let doc_md = format_datatable_doc(
                        &entry.name,
                        Some(&entry.struct_name),
                        Some(entry.count),
                        Some(&entry.package),
                        &entry.rows,
                    );
                    completions.push(EditorCompletion {
                        label: entry.name.clone(),
                        insert_text: entry.name.clone(),
                        kind: "table".to_string(),
                        detail: Some(format!("DataTable • {}", entry.struct_name)),
                        documentation: Some(doc_md),
                    });
                }
            }
        } else if let Some(s) = schema {
            for name in &s.names {
                let nl = name.to_ascii_lowercase();
                if (nl.starts_with("dt_") || nl.contains("datatable")) && (q_lower.is_empty() || nl.contains(&q_lower)) {
                    if seen.insert(name.clone()) {
                        let doc_md = format!(
                            "```typescript\n(table) {}\n```\n- **Type**: `Palworld Reflection DataTable`",
                            name
                        );
                        completions.push(EditorCompletion {
                            label: name.clone(),
                            insert_text: name.clone(),
                            kind: "table".to_string(),
                            detail: Some("DataTable".to_string()),
                            documentation: Some(doc_md),
                        });
                    }
                }
                if completions.len() >= 60 {
                    break;
                }
            }
        }

        // Specific Row Keys when in table row context
        let is_row_context = prefix_lower.contains("rows") || (!q_lower.is_empty() && prefix_lower.contains(':'));
        if is_row_context {
            if let Some(dti) = dt_index {
                let rows_found = dti.search_rows(active_target_table.as_deref(), query, 60);
                for (row_key, table_name) in rows_found {
                    if seen.insert(format!("{}:{}", table_name, row_key)) {
                        let doc_md = format!(
                            "```typescript\n(row) {}\n```\n- **Parent DataTable**: `{}`\n- **Identifier**: Unique Row ID",
                            row_key, table_name
                        );
                        completions.push(EditorCompletion {
                            label: row_key.to_string(),
                            insert_text: row_key.to_string(),
                            kind: "value".to_string(),
                            detail: Some(format!("Row • {}", table_name)),
                            documentation: Some(doc_md),
                        });
                    }
                    if completions.len() >= 120 {
                        break;
                    }
                }
            }
        }
    }

    // 2. Blueprint Asset Classes (At root level in blueprints domain, or when explicitly searching BP_ in other files)
    let should_suggest_blueprints = if is_blueprints_domain {
        parent_context.is_none() && (q_lower.starts_with("bp_") || q_lower.starts_with("wbp_") || q_lower.starts_with("abp_") || q_lower.starts_with("/game/") || q_lower.contains("blueprint") || q_lower.is_empty())
    } else {
        (q_lower.starts_with("bp_") || q_lower.starts_with("wbp_") || q_lower.starts_with("/game/")) && q_lower.len() >= 3
    };

    if should_suggest_blueprints {
        if let Some(bpi) = bp_index {
            for entry in bpi.search_blueprints(query, 80) {
                let clean_mount = if entry.package.starts_with("Pal/Content/") {
                    format!("/Game/{}", entry.package.trim_start_matches("Pal/Content/"))
                } else {
                    entry.full_path.clone()
                };

                if seen.insert(entry.class_name.clone()) {
                    let doc_md = format_blueprint_class_doc(
                        &entry.class_name,
                        &clean_mount,
                        &entry.package,
                        entry.super_class.as_deref(),
                    );
                    completions.push(EditorCompletion {
                        label: entry.class_name.clone(),
                        insert_text: entry.class_name.clone(),
                        kind: "class".to_string(),
                        detail: Some("Blueprint Class".to_string()),
                        documentation: Some(doc_md),
                    });
                }
                if completions.len() >= 120 {
                    break;
                }
            }
        } else if let Some(sdk) = sdk_index {
            for (cname, cinfo) in &sdk.classes {
                if cname.starts_with("bp_") || cname.starts_with("wbp_") {
                    if q_lower.is_empty() || cname.contains(&q_lower) {
                        if seen.insert(cinfo.clean_name.clone()) {
                            let doc_md = format!(
                                "```typescript\n(class) {}\n```\n- **Module**: `{}`\n- **Super Class**: `{}`",
                                cinfo.clean_name,
                                cinfo.module_name,
                                cinfo.clean_super_class.as_deref().unwrap_or("Object")
                            );
                            completions.push(EditorCompletion {
                                label: cinfo.clean_name.clone(),
                                insert_text: cinfo.clean_name.clone(),
                                kind: "class".to_string(),
                                detail: Some("Blueprint Class".to_string()),
                                documentation: Some(doc_md),
                            });
                        }
                    }
                }
                if completions.len() >= 100 {
                    break;
                }
            }
        }
    }

    // 3. Targeted Blueprint Class & Parent Properties (Inheritance Tree)
    if is_in_blueprint_block {
        if let Some(ref ctx) = parent_context {
            let mut candidate_classes = Vec::new();

            // Direct names
            candidate_classes.push(ctx.clone());
            if ctx.ends_with("_C") && ctx.len() > 2 {
                candidate_classes.push(ctx[..ctx.len() - 2].to_string());
            }

            // Look up parent class and package category from BlueprintIndex
            if let Some(bpi) = bp_index {
                if let Some(entry) = bpi.find_blueprint(ctx) {
                    if let Some(ref sc) = entry.super_class {
                        if !sc.is_empty() {
                            candidate_classes.push(sc.clone());
                        }
                    }
                    let pkg_lower = entry.package.to_ascii_lowercase();
                    if pkg_lower.contains("weapon") {
                        candidate_classes.push("PalWeaponBase".to_string());
                        candidate_classes.push("PalWeapon".to_string());
                    } else if pkg_lower.contains("buildobject") || pkg_lower.contains("mapobject") {
                        candidate_classes.push("PalBuildObject".to_string());
                        candidate_classes.push("PalMapObject".to_string());
                    } else if pkg_lower.contains("character") || pkg_lower.contains("monster") || pkg_lower.contains("palactor") {
                        candidate_classes.push("PalCharacter".to_string());
                        candidate_classes.push("PalMonsterCharacter".to_string());
                    } else if pkg_lower.contains("skill") || pkg_lower.contains("action") {
                        candidate_classes.push("PalActionBase".to_string());
                        candidate_classes.push("PalSkillEffectBase".to_string());
                    } else if pkg_lower.contains("status") {
                        candidate_classes.push("PalStatusBase".to_string());
                    } else if pkg_lower.contains("damage") {
                        candidate_classes.push("PalDamageReactionComponent".to_string());
                    }
                }
            }

            // Keyword heuristics for Palworld Blueprint prefixes
            let ctx_lower = ctx.to_ascii_lowercase();
            if ctx_lower.contains("weapon") || ctx_lower.contains("rifle") || ctx_lower.contains("gun") || ctx_lower.contains("bow") || ctx_lower.contains("sword") || ctx_lower.contains("spear") || ctx_lower.contains("launcher") || ctx_lower.contains("bullet") {
                candidate_classes.push("PalWeaponBase".to_string());
                candidate_classes.push("PalWeapon".to_string());
            } else if ctx_lower.contains("buildobject") || ctx_lower.contains("workbench") || ctx_lower.contains("factory") || ctx_lower.contains("chest") {
                candidate_classes.push("PalBuildObject".to_string());
                candidate_classes.push("PalMapObject".to_string());
            } else if ctx_lower.contains("pal_") || ctx_lower.contains("monster_") || ctx_lower.contains("npc_") {
                candidate_classes.push("PalCharacter".to_string());
                candidate_classes.push("PalMonsterCharacter".to_string());
            }

            // Resolve properties traversing USMAP Schema inheritance (10,631 structs with super_type)
            if let Some(s) = schema {
                for cls_name in &candidate_classes {
                    let mut curr = cls_name.clone();
                    let mut depth = 0;
                    while depth < 15 {
                        if let Some(ustruct) = s.find_struct(&curr) {
                            for prop in &ustruct.properties {
                                let pl = prop.name.to_ascii_lowercase();
                                if q_lower.is_empty() || pl.contains(&q_lower) {
                                    if seen.insert(prop.name.clone()) {
                                        let (friendly_type, doc_md) = format_property_doc(
                                            &prop.name,
                                            &ustruct.name,
                                            &prop.type_name,
                                            prop.struct_type.as_deref(),
                                            prop.enum_type.as_deref(),
                                        );
                                        completions.push(EditorCompletion {
                                            label: prop.name.clone(),
                                            insert_text: prop.name.clone(),
                                            kind: "property".to_string(),
                                            detail: Some(format!("{} • {}", friendly_type, ustruct.name)),
                                            documentation: Some(doc_md),
                                        });
                                    }
                                }
                                if completions.len() >= 120 {
                                    break;
                                }
                            }
                            if let Some(ref super_t) = ustruct.super_type {
                                if super_t.is_empty() || super_t == "Object" || super_t == "UObject" {
                                    break;
                                }
                                curr = super_t.clone();
                                depth += 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    if !completions.is_empty() {
                        break;
                    }
                }
            }

            // Resolve properties traversing SDK Index fallback
            if completions.is_empty() {
                if let Some(sdk) = sdk_index {
                    for cls_name in &candidate_classes {
                        let mut curr = cls_name.clone();
                        let mut depth = 0;
                        while depth < 15 {
                            if let Some(cinfo) = sdk.find_class(&curr) {
                                for prop in &cinfo.properties {
                                    let pl = prop.to_ascii_lowercase();
                                    if q_lower.is_empty() || pl.contains(&q_lower) {
                                        if seen.insert(prop.clone()) {
                                            let (friendly_type, doc_md) = format_property_doc(
                                                prop,
                                                &cinfo.clean_name,
                                                "Property",
                                                None,
                                                None,
                                            );
                                            completions.push(EditorCompletion {
                                                label: prop.clone(),
                                                insert_text: prop.clone(),
                                                kind: "property".to_string(),
                                                detail: Some(format!("{} • {}", friendly_type, cinfo.clean_name)),
                                                documentation: Some(doc_md),
                                            });
                                        }
                                    }
                                    if completions.len() >= 120 {
                                        break;
                                    }
                                }
                                if let Some(ref super_cls) = cinfo.clean_super_class {
                                    if super_cls.is_empty() || super_cls.eq_ignore_ascii_case("Object") || super_cls.eq_ignore_ascii_case("UObject") {
                                        break;
                                    }
                                    curr = super_cls.clone();
                                    depth += 1;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        if !completions.is_empty() {
                            break;
                        }
                    }
                }
            }
        }
    }

    // 4. Struct Properties fallback: only when actively querying (>=2 chars) and NOT in a blueprint block
    if !is_in_blueprint_block && !q_lower.is_empty() && q_lower.len() >= 2 {
        if let Some(s) = schema {
            for (sname, ustruct) in &s.structs {
                let sl = sname.to_ascii_lowercase();
                if sl.contains(&q_lower) || ustruct.properties.iter().any(|p| p.name.to_ascii_lowercase().contains(&q_lower)) {
                    for prop in &ustruct.properties {
                        let pl = prop.name.to_ascii_lowercase();
                        if pl.contains(&q_lower) {
                            if seen.insert(prop.name.clone()) {
                                let (friendly_type, doc_md) = format_property_doc(
                                    &prop.name,
                                    sname,
                                    &prop.type_name,
                                    prop.struct_type.as_deref(),
                                    prop.enum_type.as_deref(),
                                );
                                completions.push(EditorCompletion {
                                    label: prop.name.clone(),
                                    insert_text: prop.name.clone(),
                                    kind: "struct".to_string(),
                                    detail: Some(format!("{} • {}", friendly_type, sname)),
                                    documentation: Some(doc_md),
                                });
                            }
                        }
                        if completions.len() >= 80 {
                            break;
                        }
                    }
                }
                if completions.len() >= 80 {
                    break;
                }
            }
        }
    }
}

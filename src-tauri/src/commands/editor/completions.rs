use std::collections::HashSet;
use std::path::Path;
use tauri::State;
use crate::state::AppState;
use crate::usmap::{get_or_load_sdk_index, sync::get_active_usmap_path, parser::parse_usmap_file};
use super::types::EditorCompletion;

#[tauri::command]
pub async fn get_editor_completions(
    file_path: String,
    query: String,
    line_prefix: String,
    state: State<'_, AppState>,
) -> Result<Vec<EditorCompletion>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?;
    let program_path = data.settings.program_path.clone();
    let game_path = data.settings.game_path.clone();

    let mut completions = Vec::new();
    let ext = Path::new(&file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let q_lower = query.trim().to_ascii_lowercase();
    let prefix_lower = line_prefix.to_ascii_lowercase();

    let usmap_path = get_active_usmap_path(&program_path);
    let schema = if usmap_path.exists() {
        parse_usmap_file(&usmap_path).ok()
    } else {
        None
    };
    let sdk_index = get_or_load_sdk_index(&program_path, &game_path);

    if ext == "lua" {
        let is_in_path = prefix_lower.contains("registerhook")
            || prefix_lower.contains("notifyonnewobject")
            || prefix_lower.contains("staticfindobject")
            || prefix_lower.contains("findfirstof")
            || prefix_lower.contains("findallof")
            || prefix_lower.contains("/script/")
            || prefix_lower.contains("/game/")
            || q_lower.starts_with('/')
            || q_lower.starts_with("/script")
            || q_lower.starts_with("dt_")
            || q_lower.contains('.');

        if is_in_path {
            let mut seen = HashSet::new();

            // Check if user is typing a DataTable in Lua e.g. StaticFindObject("...DT_PalCharacterParameter")
            if q_lower.starts_with("dt_") {
                if let Some(ref s) = schema {
                    for name in &s.names {
                        let nl = name.to_ascii_lowercase();
                        if nl.starts_with("dt_") && (q_lower.is_empty() || nl.contains(&q_lower)) {
                            if seen.insert(name.clone()) {
                                completions.push(EditorCompletion {
                                    label: name.clone(),
                                    insert_text: name.clone(),
                                    kind: "table".to_string(),
                                    detail: Some("PalSchema DataTable".to_string()),
                                    documentation: Some(format!("Palworld reflection DataTable: {}", name)),
                                });
                            }
                        }
                        if completions.len() >= 60 {
                            break;
                        }
                    }
                }
            }

            // -----------------------------------------------------------------
            // HIERARCHICAL LEVEL 1: Root Namespaces (/ ➔ /Script/, /Game/)
            // -----------------------------------------------------------------
            if q_lower == "/" || q_lower.is_empty() {
                completions.push(EditorCompletion {
                    label: "/Script/".to_string(),
                    insert_text: "/Script/".to_string(),
                    kind: "module".to_string(),
                    detail: Some("C++ Engine & Game Modules".to_string()),
                    documentation: Some("Root namespace for native C++ modules, classes, and managers.".to_string()),
                });
                completions.push(EditorCompletion {
                    label: "/Game/".to_string(),
                    insert_text: "/Game/".to_string(),
                    kind: "module".to_string(),
                    detail: Some("Unreal Engine Blueprint Assets".to_string()),
                    documentation: Some("Root namespace for cooked game Blueprints and UI widgets.".to_string()),
                });
            }

            // -----------------------------------------------------------------
            // HIERARCHICAL LEVEL 2: Modules (/Script/ ➔ Pal., UMG., Engine.)
            // -----------------------------------------------------------------
            if let Some(ref sdk) = sdk_index {
                let is_level_2 = (q_lower.starts_with("/script/") && !q_lower.contains('.'))
                    || q_lower == "/script"
                    || q_lower == "/script/";

                if is_level_2 {
                    let mut sorted_modules: Vec<&String> = sdk.modules.iter().collect();
                    sorted_modules.sort_by(|a, b| {
                        let is_pri = |m: &str| m == "Pal" || m == "UMG" || m == "Engine" || m == "CoreUObject";
                        let a_p = if is_pri(a) { 0 } else { 1 };
                        let b_p = if is_pri(b) { 0 } else { 1 };
                        a_p.cmp(&b_p).then_with(|| a.cmp(b))
                    });

                    let mod_filter = q_lower.trim_start_matches("/script/").trim_start_matches("/script");
                    for mod_name in sorted_modules {
                        let ml = mod_name.to_ascii_lowercase();
                        if mod_filter.is_empty() || ml.starts_with(mod_filter) || ml.contains(mod_filter) {
                            let script_module = format!("/Script/{}.", mod_name);
                            if seen.insert(script_module.clone()) {
                                completions.push(EditorCompletion {
                                    label: format!("/Script/{}.", mod_name),
                                    insert_text: script_module,
                                    kind: "module".to_string(),
                                    detail: Some(format!("Module ({}.hpp)", mod_name)),
                                    documentation: Some(format!("C++ SDK Module: {}\nContains native classes and reflection structs.", mod_name)),
                                });
                            }
                        }
                    }
                }
            }

            // -----------------------------------------------------------------
            // HIERARCHICAL LEVEL 4: Methods & Delegates on a Specific Class (Class:)
            // -----------------------------------------------------------------
            if q_lower.contains(':') {
                if let Some((class_part, func_query)) = q_lower.split_once(':') {
                    let clean_cls = class_part
                        .split('.')
                        .last()
                        .unwrap_or(class_part)
                        .split('/')
                        .last()
                        .unwrap_or(class_part)
                        .trim();

                    if let Some(ref sdk) = sdk_index {
                        if let Some(cinfo) = sdk.find_class(clean_cls) {
                            let all_funcs = sdk.get_all_class_functions_and_properties(clean_cls);
                            for func in all_funcs {
                                let fl = func.to_ascii_lowercase();
                                if func_query.is_empty() || fl.contains(func_query) {
                                    let is_delegate = func.ends_with("__DelegateSignature") || func.contains("Delegate");
                                    let full_sig = format!("{}:{}", class_part, func);
                                    if seen.insert(full_sig.clone()) {
                                        completions.push(EditorCompletion {
                                            label: format!("{}:{}", cinfo.clean_name, func),
                                            insert_text: full_sig,
                                            kind: if is_delegate { "delegate".to_string() } else { "function".to_string() },
                                            detail: Some(if is_delegate { "Delegate Signature".to_string() } else { "Class Method".to_string() }),
                                            documentation: Some(format!("Member of class {}\nModule: {}", cinfo.clean_name, cinfo.module_name)),
                                        });
                                    }
                                }
                                if completions.len() >= 80 {
                                    break;
                                }
                            }
                        }
                    }
                }
            } else {
                // -----------------------------------------------------------------
                // HIERARCHICAL LEVEL 3: Classes in Specific Module (/Script/Pal. ➔ Classes)
                // -----------------------------------------------------------------
                let (target_module, class_query) = if q_lower.starts_with("/script/") {
                    let rest = q_lower.trim_start_matches("/script/");
                    if let Some((m, c)) = rest.split_once('.') {
                        (Some(m.to_string()), c.to_string())
                    } else {
                        (None, rest.to_string())
                    }
                } else if let Some((m, c)) = q_lower.split_once('.') {
                    (Some(m.to_string()), c.to_string())
                } else {
                    (None, q_lower.clone())
                };

                if let Some(ref sdk) = sdk_index {
                    for (cname, cinfo) in &sdk.classes {
                        if let Some(ref tm) = target_module {
                            if !cinfo.module_name.eq_ignore_ascii_case(tm) && !cinfo.module_name.to_ascii_lowercase().starts_with(tm) {
                                continue;
                            }
                        }

                        let clean = &cinfo.clean_name;
                        let clean_lower = clean.to_ascii_lowercase();

                        if !class_query.is_empty() && !cname.contains(&class_query) && !clean_lower.contains(&class_query) {
                            continue;
                        }

                        let script_path = format!("/Script/{}.{}", cinfo.module_name, clean);
                        if seen.insert(script_path.clone()) {
                            completions.push(EditorCompletion {
                                label: format!("/Script/{}.{}", cinfo.module_name, clean),
                                insert_text: script_path,
                                kind: "class".to_string(),
                                detail: Some(format!("{}.hpp ({} funcs)", cinfo.module_name, cinfo.functions.len())),
                                documentation: Some(format!("Native C++ class {}\nModule: {}\nSuperclass: {}", clean, cinfo.module_name, cinfo.clean_super_class.as_deref().unwrap_or("UObject"))),
                            });
                        }

                        if completions.len() >= 100 {
                            break;
                        }
                    }
                }
            }
        } else {
            // Comprehensive UE4SS Global APIs & Hooks (Official UE4SS Lua API)
            let standard_apis = [
                // Hooking & Lifecycle
                ("RegisterHook", "RegisterHook(${1:\"/Script/ModuleName.ClassName:FunctionName\"}, function(${2:Context})\n\t$0\nend)", "UE4SS Function Hook\nRegisters a pre/post hook on an Unreal Engine function."),
                ("UnregisterHook", "UnregisterHook(${1:\"/Script/ModuleName.ClassName:FunctionName\"}, ${2:PreCallbackId}, ${3:PostCallbackId})", "UE4SS Unregister Hook\nRemoves an active function hook by ID."),
                ("RegisterBeginPlayHook", "RegisterBeginPlayHook(function(${1:Context})\n\t$0\nend)", "UE4SS BeginPlay Hook\nExecutes when AActor::BeginPlay is called by Unreal Engine."),
                ("RegisterCustomEvent", "RegisterCustomEvent(${1:\"EventName\"}, function(${2:Context})\n\t$0\nend)", "UE4SS Custom Event\nRegisters a custom event hook callback."),
                ("RegisterDefaultConstructorHook", "RegisterDefaultConstructorHook(${1:\"/Script/ModuleName.ClassName\"}, function(${2:Context})\n\t$0\nend)", "UE4SS Constructor Hook\nExecutes when an object's default C++ constructor runs."),
                ("RegisterProcessConsoleExecHook", "RegisterProcessConsoleExecHook(function(${1:Context})\n\t$0\nend)", "UE4SS Console Exec Hook\nIntercepts in-game console commands before execution."),
                ("RegisterConsoleCommandHandler", "RegisterConsoleCommandHandler(${1:\"CommandName\"}, function(${2:FullCommand}, ${3:Parameters}, ${4:OutputDevice})\n\t$0\n\treturn true\nend)", "UE4SS Custom Console Command\nRegisters a custom in-game console command handler."),
                ("RegisterKeyBind", "RegisterKeyBind(${1:Key.F1}, ${2:{ ModifierKey.CONTROL \\}}, function()\n\t$0\nend)", "UE4SS Keybinding\nBinds a hotkey sequence with optional modifier keys."),
                ("NotifyOnNewObject", "NotifyOnNewObject(${1:\"/Script/ModuleName.ClassName\"}, function(${2:ConstructedObject})\n\t$0\nend)", "UE4SS Object Lifecycle Hook\nFires whenever a new instance of the specified class is allocated in memory."),

                // Object & World Discovery
                ("StaticFindObject", "StaticFindObject(${1:\"/Script/ModuleName.ClassName\"})", "Find UObject in global engine memory table by full path."),
                ("FindObject", "FindObject(${1:\"ClassName\"}, ${2:\"OuterName\"}, ${3:\"ObjectName\"})", "Find UObject by class name, outer name, and object name."),
                ("FindFirstOf", "FindFirstOf(${1:\"ClassName\"})", "Find the first active instance of a class in the loaded world."),
                ("FindAllOf", "FindAllOf(${1:\"ClassName\"})", "Find all active instances of a class in the loaded world (returns a table)."),
                ("StaticConstructObject", "StaticConstructObject(${1:Class}, ${2:Outer}, ${3:Name})", "Allocate and construct a new instance of an Unreal Engine class."),
                ("CreateInvalidObject", "CreateInvalidObject()", "Creates an invalid UObject reference placeholder."),

                // Threading & Execution
                ("ExecuteInGameThread", "ExecuteInGameThread(function()\n\t$0\nend)", "Execute code synchronously inside the main game thread."),
                ("ExecuteWithDelay", "ExecuteWithDelay(${1:1000}, function()\n\t$0\nend)", "Execute a callback after a specified delay in milliseconds."),
                ("ExecuteAsync", "ExecuteAsync(function()\n\t$0\nend)", "Execute a callback asynchronously on a background worker thread."),
                ("IsKeySequenceValid", "IsKeySequenceValid(${1:KeySequence})", "Validates whether a given key sequence is valid."),

                // Engine & Kismet Subsystems
                ("GetEngineVersion", "GetEngineVersion()", "Returns the current Unreal Engine version string."),
                ("GetWorld", "GetWorld()", "Returns the current active UWorld instance."),
                ("GetGameInstance", "GetGameInstance()", "Returns the current active UGameInstance."),
                ("GetPlayerController", "GetPlayerController()", "Returns the primary APlayerController instance."),
                ("GetKismetSystemLibrary", "GetKismetSystemLibrary()", "Returns the global UKismetSystemLibrary static helper."),
                ("GetKismetMathLibrary", "GetKismetMathLibrary()", "Returns the global UKismetMathLibrary static helper."),
                ("GetKismetStringLibrary", "GetKismetStringLibrary()", "Returns the global UKismetStringLibrary static helper."),
                ("GetKismetArrayLibrary", "GetKismetArrayLibrary()", "Returns the global UKismetArrayLibrary static helper."),
                ("GetKismetTextLibrary", "GetKismetTextLibrary()", "Returns the global UKismetTextLibrary static helper."),
                ("GetKismetGuidLibrary", "GetKismetGuidLibrary()", "Returns the global UKismetGuidLibrary static helper."),

                // Enums & Types
                ("Key", "Key.${1:F1}", "UE4SS Key Enum (e.g. Key.F1, Key.SPACE, Key.ENTER)"),
                ("ModifierKey", "ModifierKey.${1:CONTROL}", "UE4SS Modifier Key Enum (ModifierKey.CONTROL, ModifierKey.SHIFT, ModifierKey.ALT)"),
                ("FVector", "FVector(${1:0.0}, ${2:0.0}, ${3:0.0})", "Unreal Engine 3D Vector constructor"),
                ("FRotator", "FRotator(${1:0.0}, ${2:0.0}, ${3:0.0})", "Unreal Engine 3D Rotator constructor (Pitch, Yaw, Roll)"),
                ("FQuat", "FQuat(${1:0.0}, ${2:0.0}, ${3:0.0}, ${4:1.0})", "Unreal Engine Quaternion constructor"),
                ("FTransform", "FTransform(${1:Rotator}, ${2:Translation}, ${3:Scale3D})", "Unreal Engine Transform constructor"),
                ("FLinearColor", "FLinearColor(${1:1.0}, ${2:1.0}, ${3:1.0}, ${4:1.0})", "Unreal Engine RGBA Linear Color constructor"),
                ("FColor", "FColor(${1:255}, ${2:255}, ${3:255}, ${4:255})", "Unreal Engine 8-bit RGBA Color constructor"),
                ("FText", "FText(${1:\"Text\"})", "Unreal Engine Localized FText constructor"),
                ("FName", "FName(${1:\"Name\"})", "Unreal Engine FName identifier constructor"),
                ("FString", "FString(${1:\"String\"})", "Unreal Engine FString constructor"),
            ];

            for (name, snippet, doc) in &standard_apis {
                if q_lower.is_empty() || name.to_ascii_lowercase().contains(&q_lower) {
                    completions.push(EditorCompletion {
                        label: name.to_string(),
                        insert_text: snippet.to_string(),
                        kind: "api".to_string(),
                        detail: Some("UE4SS API".to_string()),
                        documentation: Some(doc.to_string()),
                    });
                }
            }
        }
    } else if ext == "json" || ext == "jsonc" {
        // -------------------------------------------------------------
        // PalSchema Dynamic DataTables, Blueprint Assets & Struct Fields
        // -------------------------------------------------------------
        let mut seen = HashSet::new();

        // 1. DataTables (DT_...)
        if let Some(ref s) = schema {
            for name in &s.names {
                let nl = name.to_ascii_lowercase();
                if (nl.starts_with("dt_") || nl.contains("datatable")) && (q_lower.is_empty() || nl.contains(&q_lower)) {
                    if seen.insert(name.clone()) {
                        completions.push(EditorCompletion {
                            label: name.clone(),
                            insert_text: name.clone(),
                            kind: "table".to_string(),
                            detail: Some("PalSchema DataTable".to_string()),
                            documentation: Some(format!("Palworld reflection DataTable: {}", name)),
                        });
                    }
                }
                if completions.len() >= 60 {
                    break;
                }
            }
        }

        // 2. Blueprint Asset Classes & Mod Paths (/Game/... or BP_...)
        if q_lower.starts_with("bp_") || q_lower.starts_with("/game/") || q_lower.contains("blueprint") || q_lower.is_empty() {
            if let Some(ref sdk) = sdk_index {
                for (cname, cinfo) in &sdk.classes {
                    if cname.starts_with("bp_") || cname.starts_with("wbp_") {
                        if q_lower.is_empty() || cname.contains(&q_lower) {
                            if seen.insert(cinfo.clean_name.clone()) {
                                completions.push(EditorCompletion {
                                    label: cinfo.clean_name.clone(),
                                    insert_text: cinfo.clean_name.clone(),
                                    kind: "class".to_string(),
                                    detail: Some("Blueprint Asset Class".to_string()),
                                    documentation: Some(format!("Game Blueprint: {}\nModule: {}", cinfo.clean_name, cinfo.module_name)),
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

        // 3. Struct Properties & Row Fields
        if let Some(ref s) = schema {
            for (sname, ustruct) in &s.structs {
                let sl = sname.to_ascii_lowercase();
                let is_row_struct = sl.ends_with("row") || sl.contains("parameter") || sl.contains("data");

                if is_row_struct || q_lower.is_empty() || sl.contains(&q_lower) {
                    for prop in &ustruct.properties {
                        let pl = prop.name.to_ascii_lowercase();
                        if q_lower.is_empty() || pl.contains(&q_lower) {
                            if seen.insert(prop.name.clone()) {
                                completions.push(EditorCompletion {
                                    label: prop.name.clone(),
                                    insert_text: prop.name.clone(),
                                    kind: "struct".to_string(),
                                    detail: Some(format!("Field of {}", sname)),
                                    documentation: Some(format!("Property: {}\nParent Struct: {}\nType: {}", prop.name, sname, prop.type_name)),
                                });
                            }
                        }
                        if completions.len() >= 100 {
                            break;
                        }
                    }
                }
                if completions.len() >= 100 {
                    break;
                }
            }
        }
    }

    Ok(completions)
}

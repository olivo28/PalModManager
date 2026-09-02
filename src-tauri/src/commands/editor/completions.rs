use std::collections::HashSet;
use std::path::Path;
use tauri::State;
use crate::state::AppState;
use crate::usmap::{get_or_load_sdk_index, get_or_load_schema, get_or_load_datatable_index, get_or_load_blueprint_index};
use super::types::EditorCompletion;

#[tauri::command]
pub async fn get_editor_completions(
    file_path: String,
    query: String,
    line_prefix: String,
    mod_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<EditorCompletion>, String> {
    let (program_path, game_path, mod_info_opt) = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        let mod_info = mod_id.as_ref().and_then(|mid| data.mods.iter().find(|m| m.id == *mid).cloned());
        (data.settings.program_path.clone(), data.settings.game_path.clone(), mod_info)
    };

    let mut completions = Vec::new();

    // 1. Workspace Content-Aware Mod Symbols (.jsonc, .json, .lua)
    if let Some(ref mod_info) = mod_info_opt {
        if let Ok(file_list) = crate::commands::config_commands::list_mod_files(mod_info.id.clone(), state.clone()) {
            if !file_list.is_empty() {
                let symbols = super::workspace_index::get_or_build_mod_symbols(mod_info, &file_list);
                let ws_completions = super::workspace_index::find_workspace_completions(&symbols, &query, 60);
                completions.extend(ws_completions);
            }
        }
    }

    let ext = Path::new(&file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let q_lower = query.trim().to_ascii_lowercase();
    let prefix_lower = line_prefix.to_ascii_lowercase();

    let schema = get_or_load_schema(&program_path);
    let sdk_index = get_or_load_sdk_index(&program_path, &game_path);
    let dt_index = get_or_load_datatable_index(&program_path);
    let bp_index = get_or_load_blueprint_index(&program_path);

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
                if let Some(ref dti) = dt_index {
                    for entry in dti.search_tables(&query, 60) {
                        if seen.insert(entry.name.clone()) {
                            completions.push(EditorCompletion {
                                label: entry.name.clone(),
                                insert_text: entry.name.clone(),
                                kind: "table".to_string(),
                                detail: Some(format!("DataTable ({}, {} rows)", entry.struct_name, entry.count)),
                                documentation: Some(format!("Package: {}\nRowStruct: {}\nTotal Rows: {}", entry.package, entry.struct_name, entry.count)),
                            });
                        }
                    }
                } else if let Some(ref s) = schema {
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
            // Comprehensive UE4SS Global APIs, Hooks & Modern Delayed Action System (Official UE4SS Lua API)
            let standard_apis = [
                // Hooking & Lifecycle
                ("RegisterHook", "RegisterHook(${1:\"/Script/ModuleName.ClassName:FunctionName\"}, function(${2:self}, ${3:Param1})\n\t$0\nend)", "UE4SS Function Hook\nRegisters a pre/post hook on a UFunction. Returns preId, postId."),
                ("UnregisterHook", "UnregisterHook(${1:\"/Script/ModuleName.ClassName:FunctionName\"}, ${2:PreCallbackId}, ${3:PostCallbackId})", "UE4SS Unregister Hook\nRemoves an active function hook by ID."),
                ("NotifyOnNewObject", "NotifyOnNewObject(${1:\"/Script/ModuleName.ClassName\"}, function(${2:ConstructedObject})\n\t$0\nend)", "UE4SS Object Lifecycle Hook\nFires whenever a new instance of the specified class is allocated in memory."),
                ("RegisterCustomEvent", "RegisterCustomEvent(${1:\"EventName\"}, function(${2:Context})\n\t$0\nend)", "UE4SS Custom Event\nRegisters a callback called when a BP event fires with EventName."),
                ("RegisterKeyBind", "RegisterKeyBind(${1:Key.F1}, ${2:{ ModifierKey.CONTROL \\}}, function()\n\t$0\nend)", "UE4SS Keybinding\nBinds a hotkey sequence with optional modifier keys."),
                ("IsKeyBindRegistered", "IsKeyBindRegistered(${1:Key.F1}, ${2:{ ModifierKey.CONTROL \\}})", "UE4SS Keybind Checker\nReturns true if the key combination is already registered."),
                ("RegisterLoadMapPreHook", "RegisterLoadMapPreHook(function(${1:Engine}, ${2:WorldContext}, ${3:URL}, ${4:PendingGame}, ${5:Error})\n\t$0\nend)", "UE4SS LoadMap Pre Hook\nExecutes before UEngine::LoadMap is called."),
                ("RegisterLoadMapPostHook", "RegisterLoadMapPostHook(function(${1:Engine}, ${2:WorldContext}, ${3:URL}, ${4:PendingGame}, ${5:Error})\n\t$0\nend)", "UE4SS LoadMap Post Hook\nExecutes after UEngine::LoadMap is called."),
                ("RegisterInitGameStatePreHook", "RegisterInitGameStatePreHook(function(${1:Context})\n\t$0\nend)", "UE4SS InitGameState Pre Hook\nExecutes before AGameModeBase::InitGameState runs."),
                ("RegisterInitGameStatePostHook", "RegisterInitGameStatePostHook(function(${1:Context})\n\t$0\nend)", "UE4SS InitGameState Post Hook\nExecutes after AGameModeBase::InitGameState runs."),
                ("RegisterBeginPlayPreHook", "RegisterBeginPlayPreHook(function(${1:Context})\n\t$0\nend)", "UE4SS BeginPlay Pre Hook\nExecutes before AActor::BeginPlay is called by Unreal Engine."),
                ("RegisterBeginPlayPostHook", "RegisterBeginPlayPostHook(function(${1:Context})\n\t$0\nend)", "UE4SS BeginPlay Post Hook\nExecutes after AActor::BeginPlay is called by Unreal Engine."),
                ("RegisterProcessConsoleExecPreHook", "RegisterProcessConsoleExecPreHook(function(${1:Context}, ${2:Cmd}, ${3:Parts}, ${4:Ar}, ${5:Executor})\n\t$0\nend)", "UE4SS Console Exec Pre Hook\nIntercepts in-game console commands before execution."),
                ("RegisterProcessConsoleExecPostHook", "RegisterProcessConsoleExecPostHook(function(${1:Context}, ${2:Cmd}, ${3:Parts}, ${4:Ar}, ${5:Executor})\n\t$0\nend)", "UE4SS Console Exec Post Hook\nIntercepts in-game console commands after execution."),
                ("RegisterCallFunctionByNameWithArgumentsPreHook", "RegisterCallFunctionByNameWithArgumentsPreHook(function(${1:Context}, ${2:Str}, ${3:Ar}, ${4:Executor}, ${5:bForceCallWithNonExec})\n\t$0\nend)", "UE4SS CallFunctionByName Pre Hook\nExecutes before CallFunctionByNameWithArguments."),
                ("RegisterCallFunctionByNameWithArgumentsPostHook", "RegisterCallFunctionByNameWithArgumentsPostHook(function(${1:Context}, ${2:Str}, ${3:Ar}, ${4:Executor}, ${5:bForceCallWithNonExec})\n\t$0\nend)", "UE4SS CallFunctionByName Post Hook\nExecutes after CallFunctionByNameWithArguments."),
                ("RegisterULocalPlayerExecPreHook", "RegisterULocalPlayerExecPreHook(function(${1:Context}, ${2:InWorld}, ${3:Cmd}, ${4:Ar})\n\t$0\nend)", "UE4SS ULocalPlayer Exec Pre Hook\nExecutes before ULocalPlayer::Exec runs."),
                ("RegisterULocalPlayerExecPostHook", "RegisterULocalPlayerExecPostHook(function(${1:Context}, ${2:InWorld}, ${3:Cmd}, ${4:Ar})\n\t$0\nend)", "UE4SS ULocalPlayer Exec Post Hook\nExecutes after ULocalPlayer::Exec runs."),
                ("RegisterConsoleCommandHandler", "RegisterConsoleCommandHandler(${1:\"CommandName\"}, function(${2:Cmd}, ${3:CommandParts}, ${4:Ar})\n\t$0\n\treturn true\nend)", "UE4SS Custom Console Command\nRegisters a custom console command in UGameViewportClient context."),
                ("RegisterConsoleCommandGlobalHandler", "RegisterConsoleCommandGlobalHandler(${1:\"CommandName\"}, function(${2:Cmd}, ${3:CommandParts}, ${4:Ar})\n\t$0\n\treturn true\nend)", "UE4SS Global Console Command\nRegisters a custom console command across all contexts."),
                ("RegisterCustomProperty", "RegisterCustomProperty(${1:CustomPropertyInfo})", "UE4SS Custom Property\nRegisters a custom property metadata table for UObject access."),
                ("ForEachUObject", "ForEachUObject(function(${1:Object}, ${2:ChunkIndex}, ${3:ObjectIndex})\n\t$0\nend)", "UE4SS GUObjectArray Iterator\nIterates every valid UObject in engine memory."),

                // Modern Delayed Action System (UE4SS v3 / v4+)
                ("LoopInGameThreadWithDelay", "LoopInGameThreadWithDelay(${1:1000}, function()\n\t$0\n\treturn false -- return true to stop loop\nend)", "UE4SS Modern Repeating Timer\nExecutes a repeating callback on the game thread with support for pause/resume/cancel."),
                ("ExecuteInGameThreadWithDelay", "ExecuteInGameThreadWithDelay(${1:500}, function()\n\t$0\nend)", "UE4SS Delayed Game Thread Execution\nSchedules a callback to run on the game thread after delay milliseconds."),
                ("RetriggerableExecuteInGameThreadWithDelay", "RetriggerableExecuteInGameThreadWithDelay(${1:actionHandle}, ${2:500}, function()\n\t$0\nend)", "UE4SS Debounced Delayed Action\nResets the timer if called again with the same handle before expiry."),
                ("MakeActionHandle", "MakeActionHandle()", "Generates a unique handle for Delayed Actions."),
                ("CancelDelayedAction", "CancelDelayedAction(${1:actionHandle})", "Cancels an active delayed action timer."),
                ("PauseDelayedAction", "PauseDelayedAction(${1:actionHandle})", "Pauses an active delayed action timer without cancelling it."),
                ("UnpauseDelayedAction", "UnpauseDelayedAction(${1:actionHandle})", "Resumes a paused delayed action timer."),
                ("ResetDelayedActionTimer", "ResetDelayedActionTimer(${1:actionHandle})", "Resets the elapsed time counter for a delayed action."),
                ("SetDelayedActionTimer", "SetDelayedActionTimer(${1:actionHandle}, ${2:newDelayMs})", "Changes the delay rate of an active delayed action timer."),
                ("ClearAllDelayedActions", "ClearAllDelayedActions()", "Cancels all delayed actions owned by this mod."),
                ("IsDelayedActionActive", "IsDelayedActionActive(${1:actionHandle})", "Returns true if the delayed action is currently pending or running."),
                ("IsDelayedActionPaused", "IsDelayedActionPaused(${1:actionHandle})", "Returns true if the delayed action is paused."),
                ("GetDelayedActionTimeRemaining", "GetDelayedActionTimeRemaining(${1:actionHandle})", "Returns remaining time in milliseconds before timer fires."),

                // Object & World Discovery
                ("StaticFindObject", "StaticFindObject(${1:\"/Script/ModuleName.ClassName\"})", "Find UObject in global engine memory table by full path."),
                ("FindObject", "FindObject(${1:\"ClassName\"}, ${2:\"OuterName\"}, ${3:\"ObjectName\"})", "Find UObject by class name, outer name, and object name."),
                ("FindFirstOf", "FindFirstOf(${1:\"ClassName\"})", "Find the first active instance of a class in the loaded world."),
                ("FindAllOf", "FindAllOf(${1:\"ClassName\"})", "Find all active instances of a class in the loaded world (returns a table)."),
                ("StaticConstructObject", "StaticConstructObject(${1:Class}, ${2:Outer}, ${3:Name})", "Allocate and construct a new instance of an Unreal Engine class."),
                ("CreateInvalidObject", "CreateInvalidObject()", "Creates an invalid UObject reference placeholder where :IsValid() returns false."),
                ("LoadAsset", "LoadAsset(${1:\"/Game/Path/Asset.Asset\"})", "Synchronously loads a game asset by internal path (game thread only)."),
                ("IterateGameDirectories", "IterateGameDirectories()", "Returns a nested table of game directory structures and files."),

                // Execution & Diagnostics
                ("ExecuteInGameThread", "ExecuteInGameThread(function()\n\t$0\nend)", "Execute code synchronously inside the main game thread using ProcessEvent."),
                ("print", "print(${1:\"Message\"})", "Outputs text to the UE4SS developer console."),
                ("DumpAllObjects", "DumpAllObjects()", "Generates a complete dump of all loaded UObjects to disk."),
                ("GenerateSDK", "GenerateSDK()", "Generates C++ SDK headers for the active game instance."),
                ("GenerateLuaTypes", "GenerateLuaTypes()", "Generates EmmyLua/LuaLS type definitions for editor IntelliSense."),
                ("DumpUSMAP", "DumpUSMAP()", "Generates an unversioned .usmap schema mapping file for the current game patch."),
                ("RestartCurrentMod", "RestartCurrentMod()", "Hot-reloads the current Lua mod on the next update cycle."),
                ("RestartMod", "RestartMod(${1:\"ModName\"})", "Hot-reloads another UE4SS mod by folder name."),

                // Enums & Types
                ("Key", "Key.${1:F1}", "UE4SS Key Enum (e.g. Key.F1, Key.SPACE, Key.ENTER, Key.TAB, Key.ESCAPE, Key.A)"),
                ("ModifierKey", "ModifierKey.${1:CONTROL}", "UE4SS Modifier Key Enum (ModifierKey.CONTROL, ModifierKey.SHIFT, ModifierKey.ALT, ModifierKey.PRIMARY)"),
                ("EObjectFlags", "EObjectFlags.${1:RF_Public}", "Unreal Engine Object Flags bitmask (RF_Public, RF_Standalone, RF_Transient, RF_ClassDefaultObject, RF_AllFlags)"),
                ("EInternalObjectFlags", "EInternalObjectFlags.${1:Native}", "Unreal Engine Internal Object Flags bitmask (Native, Async, ReachableInCluster, RootSet)"),
                ("PropertyTypes", "PropertyTypes.${1:ObjectProperty}", "UE4SS Property Type identifiers (ObjectProperty, IntProperty, FloatProperty, StrProperty, BoolProperty, ArrayProperty, MapProperty, StructProperty)"),
                ("UnrealVersion", "UnrealVersion.GetMajor()", "UE4SS Engine Version Inspector (GetMajor(), GetMinor(), IsAtLeast(5, 1))"),
                ("UE4SS", "UE4SS.GetVersion()", "UE4SS Metadata Object (GetVersion() -> major, minor, hotfix)"),
                ("Mod", "Mod:SetSharedVariable(${1:\"Key\"}, ${2:Value})", "Mod Cross-Communication Interface (SetSharedVariable, GetSharedVariable)"),

                // Math & Struct Types
                ("FVector", "FVector(${1:0.0}, ${2:0.0}, ${3:0.0})", "Unreal Engine 3D Vector constructor (X, Y, Z)"),
                ("FRotator", "FRotator(${1:0.0}, ${2:0.0}, ${3:0.0})", "Unreal Engine 3D Rotator constructor (Pitch, Yaw, Roll)"),
                ("FQuat", "FQuat(${1:0.0}, ${2:0.0}, ${3:0.0}, ${4:1.0})", "Unreal Engine Quaternion constructor (X, Y, Z, W)"),
                ("FTransform", "FTransform(${1:Rotator}, ${2:Translation}, ${3:Scale3D})", "Unreal Engine Transform constructor"),
                ("FLinearColor", "FLinearColor(${1:1.0}, ${2:1.0}, ${3:1.0}, ${4:1.0})", "Unreal Engine RGBA Linear Color constructor (R, G, B, A)"),
                ("FColor", "FColor(${1:255}, ${2:255}, ${3:255}, ${4:255})", "Unreal Engine 8-bit RGBA Color constructor"),
                ("FText", "FText(${1:\"Text\"})", "Unreal Engine Localized FText constructor"),
                ("FName", "FName(${1:\"Name\"})", "Unreal Engine FName identifier constructor"),
                ("FString", "FString(${1:\"String\"})", "Unreal Engine FString constructor"),

                // Best-Practice Defensive Patterns & Safe Error Handling
                ("safe_object_check", "if ${1:obj} and ${1:obj}:IsValid() then\n\t$0\nend", "Defensive check verifying an Unreal object is not nil and currently valid in engine memory."),
                ("safe_pcall", "local ok, err = pcall(function()\n\t$0\nend)\nif not ok then\n\tprint(string.format(\"[ERROR] Execution failed: %s\", tostring(err)))\nend", "Protected call with explicit error capture and developer console visibility."),
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

        // 0. PalSchema Starter Snippet
        if q_lower.is_empty() || "palschema".starts_with(&q_lower) || "datatable".starts_with(&q_lower) {
            completions.push(EditorCompletion {
                label: "PalSchema DataTable Patch".to_string(),
                insert_text: "{\n  \"DataTable\": \"${1:DT_ItemData}\",\n  \"Rows\": {\n    \"${2:RowName}\": {\n      $0\n    }\n  }\n}".to_string(),
                kind: "api".to_string(),
                detail: Some("PalSchema Template".to_string()),
                documentation: Some("Starter scaffold for modifying game DataTables via PalSchema runtime JSON injection.".to_string()),
            });
        }

        // 1. DataTables (DT_...) from DataTableIndex or USMAP Schema
        let mut active_target_table: Option<String> = None;

        // Check if line_prefix contains a referenced DataTable e.g. "DataTable": "DT_ItemData"
        if let Some(pos) = prefix_lower.find("dt_") {
            let slice = &line_prefix[pos..];
            let end_pos = slice.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(slice.len());
            active_target_table = Some(slice[..end_pos].to_string());
        }

        if let Some(ref dti) = dt_index {
            for entry in dti.search_tables(&query, 60) {
                if seen.insert(entry.name.clone()) {
                    completions.push(EditorCompletion {
                        label: entry.name.clone(),
                        insert_text: entry.name.clone(),
                        kind: "table".to_string(),
                        detail: Some(format!("DataTable ({}, {} rows)", entry.struct_name, entry.count)),
                        documentation: Some(format!("Package: {}\nRowStruct: {}\nTotal Rows: {}\nSample Rows: {}", entry.package, entry.struct_name, entry.count, entry.rows.iter().take(5).cloned().collect::<Vec<_>>().join(", "))),
                    });
                }
            }
        } else if let Some(ref s) = schema {
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

        // 2. Specific Row Keys (e.g. Sphere_Mega, Meat_SheepBall, Boss_Anubis)
        let is_row_context = prefix_lower.contains("rows")
            || prefix_lower.contains('{')
            || !q_lower.is_empty();

        if is_row_context {
            if let Some(ref dti) = dt_index {
                let rows_found = dti.search_rows(active_target_table.as_deref(), &query, 60);
                for (row_key, table_name) in rows_found {
                    if seen.insert(format!("{}:{}", table_name, row_key)) {
                        completions.push(EditorCompletion {
                            label: row_key.to_string(),
                            insert_text: row_key.to_string(),
                            kind: "value".to_string(),
                            detail: Some(format!("Row in {}", table_name)),
                            documentation: Some(format!("DataTable Row Identifier\nKey: {}\nParent Table: {}", row_key, table_name)),
                        });
                    }
                    if completions.len() >= 120 {
                        break;
                    }
                }
            }
        }

        let (parent_context, _) = if line_prefix.starts_with("[context:") {
            if let Some((ctx, _)) = line_prefix.split_once(']') {
                (Some(ctx.trim_start_matches("[context:").trim().to_string()), ())
            } else {
                (None, ())
            }
        } else {
            (None, ())
        };

        let is_in_blueprint_block = parent_context.as_ref().map(|ctx| {
            let cl = ctx.to_ascii_lowercase();
            cl.starts_with("bp_") || cl.starts_with("wbp_") || cl.starts_with("abp_")
        }).unwrap_or(false);

        // 2. Blueprint Asset Classes & Mod Paths (/Game/... or BP_...) - only when not inside a property block
        if !is_in_blueprint_block && (q_lower.starts_with("bp_") || q_lower.starts_with("wbp_") || q_lower.starts_with("abp_") || q_lower.starts_with("/game/") || q_lower.contains("blueprint") || q_lower.is_empty()) {
            if let Some(ref bpi) = bp_index {
                for entry in bpi.search_blueprints(&query, 80) {
                    let clean_mount = if entry.package.starts_with("Pal/Content/") {
                        format!("/Game/{}", entry.package.trim_start_matches("Pal/Content/"))
                    } else {
                        entry.full_path.clone()
                    };

                    if seen.insert(entry.class_name.clone()) {
                        completions.push(EditorCompletion {
                            label: entry.class_name.clone(),
                            insert_text: entry.class_name.clone(),
                            kind: "class".to_string(),
                            detail: Some(clean_mount.clone()),
                            documentation: Some(format!("Live Game Blueprint Asset: {}\nMount Path: {}\nSource Package: {}", entry.class_name, clean_mount, entry.package)),
                        });
                    }
                    if seen.insert(entry.name.clone()) {
                        completions.push(EditorCompletion {
                            label: entry.name.clone(),
                            insert_text: entry.name.clone(),
                            kind: "class".to_string(),
                            detail: Some(clean_mount.clone()),
                            documentation: Some(format!("Live Game Blueprint Asset: {}\nMount Path: {}\nClass: {}", entry.name, clean_mount, entry.class_name)),
                        });
                    }
                    if completions.len() >= 120 {
                        break;
                    }
                }
            } else if let Some(ref sdk) = sdk_index {
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

        // 3. Struct Properties & Engine Reflection Fields
        if let Some(ref s) = schema {
            for (sname, ustruct) in &s.structs {
                let sl = sname.to_ascii_lowercase();
                let is_row_struct = sl.ends_with("row") || sl.contains("parameter") || sl.contains("data");

                // If inside a blueprint block or user is searching specifically, inspect properties across all structs
                let should_scan_properties = is_in_blueprint_block || is_row_struct || q_lower.is_empty() || sl.contains(&q_lower) || (!q_lower.is_empty() && q_lower.len() >= 3);

                if should_scan_properties {
                    for prop in &ustruct.properties {
                        let pl = prop.name.to_ascii_lowercase();
                        if q_lower.is_empty() || pl.contains(&q_lower) {
                            if seen.insert(prop.name.clone()) {
                                let (friendly_type, doc_hint) = get_friendly_type_and_doc(
                                    &prop.type_name,
                                    prop.struct_type.as_deref(),
                                    prop.enum_type.as_deref(),
                                );
                                completions.push(EditorCompletion {
                                    label: prop.name.clone(),
                                    insert_text: prop.name.clone(),
                                    kind: "struct".to_string(),
                                    detail: Some(format!("{} • {}", friendly_type, sname)),
                                    documentation: Some(format!("Property: {}\nParent Struct: {}\nType: {}\n{}", prop.name, sname, prop.type_name, doc_hint)),
                                });
                            }
                        }
                        if completions.len() >= 120 {
                            break;
                        }
                    }
                }
                if completions.len() >= 120 {
                    break;
                }
            }
        }
    }

    Ok(completions)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReflectionCatalogsStatus {
    pub total_datatables: usize,
    pub total_datatable_rows: usize,
    pub datatables_active_file: String,
    pub total_blueprints: usize,
    pub blueprints_build_id: String,
    pub blueprints_game_ver: String,
    pub blueprints_filename: String,
    pub blueprints_sha256: String,
    pub blueprints_size: u64,
}

#[tauri::command]
pub async fn get_reflection_catalogs_status(
    state: State<'_, AppState>,
) -> Result<ReflectionCatalogsStatus, String> {
    let program_path = {
        let data = state.data.lock().map_err(|e| e.to_string())?;
        data.settings.program_path.clone()
    };

    let dt = crate::usmap::get_or_load_datatable_index(&program_path);
    let bp = crate::usmap::get_or_load_blueprint_index(&program_path);

    let bp_dir = crate::usmap::get_blueprints_dir(&program_path);
    let mut bp_ver = "v1.0.3".to_string();
    let mut bp_build = "24575825".to_string();
    let mut bp_file = "Palworld_Blueprints_24575825.json".to_string();
    let mut bp_hash = "c8ec30d2888b12251dc8087622f9ea502011539d2aad9f5a4c4617ec1de97528".to_string();
    let mut bp_size: u64 = 7549048;

    let manifest_path = crate::usmap::sync::find_bundled_resource("resources/blueprints/manifest.json")
        .or_else(|| {
            let p = bp_dir.join("manifest.json");
            if p.exists() { Some(p) } else { None }
        });

    if let Some(mp) = manifest_path {
        if let Ok(m_str) = std::fs::read_to_string(&mp) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&m_str) {
                if let Some(list) = v.get("blueprints").and_then(|a| a.as_array()) {
                    if let Some(first) = list.first() {
                        if let Some(fname) = first.get("blueprints_filename").and_then(|s| s.as_str()) {
                            bp_file = fname.to_string();
                        }
                        if let Some(ver) = first.get("game_version").and_then(|s| s.as_str()) {
                            bp_ver = ver.to_string();
                        }
                        if let Some(b) = first.get("steam_build_id").and_then(|s| s.as_str()) {
                            bp_build = b.to_string();
                        }
                        if let Some(h) = first.get("sha256").and_then(|s| s.as_str()) {
                            bp_hash = h.to_string();
                        }
                        if let Some(sz) = first.get("file_size_bytes").and_then(|s| s.as_u64()) {
                            bp_size = sz;
                        }
                    }
                }
            }
        }
    }

    Ok(ReflectionCatalogsStatus {
        total_datatables: dt.as_ref().map(|d| d.total_tables).unwrap_or(0),
        total_datatable_rows: dt.as_ref().map(|d| d.total_rows).unwrap_or(0),
        datatables_active_file: "dt_index.json".to_string(),
        total_blueprints: bp.as_ref().map(|b| b.total_blueprints).unwrap_or(0),
        blueprints_build_id: bp_build,
        blueprints_game_ver: bp_ver,
        blueprints_filename: bp_file,
        blueprints_sha256: bp_hash,
        blueprints_size: bp_size,
    })
}

fn get_friendly_type_and_doc(type_name: &str, struct_type: Option<&str>, enum_type: Option<&str>) -> (String, String) {
    match type_name {
        "BoolProperty" => (
            "boolean".to_string(),
            "Expected: true or false\nType: Boolean (BoolProperty)".to_string(),
        ),
        "FloatProperty" => (
            "float".to_string(),
            "Expected: decimal number (e.g. 1.0, 150.5)\nType: Float (FloatProperty)".to_string(),
        ),
        "DoubleProperty" => (
            "double".to_string(),
            "Expected: floating-point number\nType: Double (DoubleProperty)".to_string(),
        ),
        "IntProperty" | "Int32Property" => (
            "integer".to_string(),
            "Expected: whole number (e.g. 10, 500)\nType: Integer (IntProperty)".to_string(),
        ),
        "Int64Property" => (
            "int64".to_string(),
            "Expected: 64-bit integer\nType: Int64 (Int64Property)".to_string(),
        ),
        "Int16Property" => (
            "int16".to_string(),
            "Expected: 16-bit integer\nType: Int16 (Int16Property)".to_string(),
        ),
        "Int8Property" => (
            "int8".to_string(),
            "Expected: 8-bit integer (-128 to 127)\nType: Int8 (Int8Property)".to_string(),
        ),
        "UInt32Property" => (
            "uint32".to_string(),
            "Expected: unsigned integer (>= 0)\nType: UInt32 (UInt32Property)".to_string(),
        ),
        "UInt16Property" => (
            "uint16".to_string(),
            "Expected: unsigned 16-bit integer (>= 0)\nType: UInt16 (UInt16Property)".to_string(),
        ),
        "UInt64Property" => (
            "uint64".to_string(),
            "Expected: unsigned 64-bit integer\nType: UInt64 (UInt64Property)".to_string(),
        ),
        "ByteProperty" => (
            "byte".to_string(),
            "Expected: byte integer (0 to 255)\nType: Byte (ByteProperty)".to_string(),
        ),
        "StrProperty" => (
            "string".to_string(),
            "Expected: \"text\"\nType: String (StrProperty)".to_string(),
        ),
        "NameProperty" => (
            "name".to_string(),
            "Expected: \"IdentifierName\"\nType: FName (NameProperty)".to_string(),
        ),
        "TextProperty" => (
            "text".to_string(),
            "Expected: \"Localized text\"\nType: FText (TextProperty)".to_string(),
        ),
        "ArrayProperty" => (
            "array".to_string(),
            "Expected: [ ... ] (Array list)\nType: TArray (ArrayProperty)".to_string(),
        ),
        "MapProperty" => (
            "map".to_string(),
            "Expected: { ... } (Key-Value map)\nType: TMap (MapProperty)".to_string(),
        ),
        "SetProperty" => (
            "set".to_string(),
            "Expected: [ ... ] (Unique elements)\nType: TSet (SetProperty)".to_string(),
        ),
        "StructProperty" => {
            let sname = struct_type.unwrap_or("Struct");
            (
                format!("struct<{}>", sname),
                format!("Expected: {{ ... }} (Object)\nType: F{} (StructProperty)", sname),
            )
        }
        "EnumProperty" => {
            let ename = enum_type.unwrap_or("Enum");
            (
                format!("enum<{}>", ename),
                format!("Expected: \"{}::Value\"\nType: Enum (EnumProperty)", ename),
            )
        }
        "ObjectProperty" | "WeakObjectProperty" | "SoftObjectProperty" => (
            "object".to_string(),
            "Expected: \"/Game/Path/Asset.Asset_C\" or null\nType: UObject Reference".to_string(),
        ),
        other => (
            other.trim_end_matches("Property").to_ascii_lowercase(),
            format!("Type: {}", other),
        ),
    }
}

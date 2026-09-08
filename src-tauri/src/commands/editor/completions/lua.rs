use std::collections::HashSet;
use std::sync::Arc;
use crate::usmap::{DataTableIndex, SdkIndex, UsmapSchema};
use crate::commands::editor::types::EditorCompletion;

pub fn populate_lua_completions(
    query: &str,
    line_prefix: &str,
    schema: Option<&Arc<UsmapSchema>>,
    sdk_index: Option<&Arc<SdkIndex>>,
    dt_index: Option<&Arc<DataTableIndex>>,
    lua_signatures: Option<&Arc<std::collections::HashMap<String, std::collections::HashMap<String, crate::usmap::LuaMethodInfo>>>>,
    completions: &mut Vec<EditorCompletion>,
    seen: &mut HashSet<String>,
) {
    let q_lower = query.trim().to_ascii_lowercase();
    let prefix_lower = line_prefix.to_ascii_lowercase();

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
        // Check if user is typing a DataTable in Lua e.g. StaticFindObject("...DT_PalCharacterParameter")
        if q_lower.starts_with("dt_") {
            if let Some(dti) = dt_index {
                for entry in dti.search_tables(query, 60) {
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
            } else if let Some(s) = schema {
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

        // HIERARCHICAL LEVEL 1: Root Namespaces (/ ➔ /Script/, /Game/)
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

        // HIERARCHICAL LEVEL 2: Modules (/Script/ ➔ Pal., UMG., Engine.)
        if let Some(sdk) = sdk_index {
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

        // HIERARCHICAL LEVEL 4: Methods & Delegates on a Specific Class (Class:)
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

                if let Some(sdk) = sdk_index {
                    if let Some(cinfo) = sdk.find_class(clean_cls) {
                        let all_funcs = sdk.get_all_class_functions_and_properties(clean_cls);
                        for func in all_funcs {
                            let fl = func.to_ascii_lowercase();
                            if func_query.is_empty() || fl.contains(func_query) {
                                let is_delegate = func.ends_with("__DelegateSignature") || func.contains("Delegate");
                                let full_sig = format!("{}:{}", class_part, func);
                                if seen.insert(full_sig.clone()) {
                                    let (insert_text, detail_override) = if let Some(sig_map) = lua_signatures {
                                        if let Some(method_info) = crate::usmap::find_lua_method_signature(sig_map, &cinfo.clean_name, &func) {
                                            (method_info.build_snippet(class_part), Some(method_info.build_detail()))
                                        } else {
                                            (full_sig, None)
                                        }
                                    } else {
                                        (full_sig, None)
                                    };

                                    let detail_text = detail_override.unwrap_or_else(|| {
                                        if is_delegate {
                                            "Delegate Signature".to_string()
                                        } else {
                                            "Class Method".to_string()
                                        }
                                    });

                                    completions.push(EditorCompletion {
                                        label: format!("{}:{}", cinfo.clean_name, func),
                                        insert_text,
                                        kind: if is_delegate { "delegate".to_string() } else { "function".to_string() },
                                        detail: Some(detail_text),
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
            // HIERARCHICAL LEVEL 3: Classes in Specific Module (/Script/Pal. ➔ Classes)
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

            if let Some(sdk) = sdk_index {
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
            ("StaticFindObject", "StaticFindObject(${1:\"/Script/ModuleName.ClassName\"})", "Find UObject in global engine memory table by full path."),
            ("FindObject", "FindObject(${1:\"ClassName\"}, ${2:\"OuterName\"}, ${3:\"ObjectName\"})", "Find UObject by class name, outer name, and object name."),
            ("FindFirstOf", "FindFirstOf(${1:\"ClassName\"})", "Find the first active instance of a class in the loaded world."),
            ("FindAllOf", "FindAllOf(${1:\"ClassName\"})", "Find all active instances of a class in the loaded world (returns a table)."),
            ("StaticConstructObject", "StaticConstructObject(${1:Class}, ${2:Outer}, ${3:Name})", "Allocate and construct a new instance of an Unreal Engine class."),
            ("CreateInvalidObject", "CreateInvalidObject()", "Creates an invalid UObject reference placeholder where :IsValid() returns false."),
            ("LoadAsset", "LoadAsset(${1:\"/Game/Path/Asset.Asset\"})", "Synchronously loads a game asset by internal path (game thread only)."),
            ("IterateGameDirectories", "IterateGameDirectories()", "Returns a nested table of game directory structures and files."),
            ("ExecuteInGameThread", "ExecuteInGameThread(function()\n\t$0\nend)", "Execute code synchronously inside the main game thread using ProcessEvent."),
            ("print", "print(${1:\"Message\"})", "Outputs text to the UE4SS developer console."),
            ("DumpAllObjects", "DumpAllObjects()", "Generates a complete dump of all loaded UObjects to disk."),
            ("GenerateSDK", "GenerateSDK()", "Generates C++ SDK headers for the active game instance."),
            ("GenerateLuaTypes", "GenerateLuaTypes()", "Generates EmmyLua/LuaLS type definitions for editor IntelliSense."),
            ("DumpUSMAP", "DumpUSMAP()", "Generates an unversioned .usmap schema mapping file for the current game patch."),
            ("RestartCurrentMod", "RestartCurrentMod()", "Hot-reloads the current Lua mod on the next update cycle."),
            ("RestartMod", "RestartMod(${1:\"ModName\"})", "Hot-reloads another UE4SS mod by folder name."),
            ("Key", "Key.${1:F1}", "UE4SS Key Enum (e.g. Key.F1, Key.SPACE, Key.ENTER, Key.TAB, Key.ESCAPE, Key.A)"),
            ("ModifierKey", "ModifierKey.${1:CONTROL}", "UE4SS Modifier Key Enum (ModifierKey.CONTROL, ModifierKey.SHIFT, ModifierKey.ALT, ModifierKey.PRIMARY)"),
            ("EObjectFlags", "EObjectFlags.${1:RF_Public}", "Unreal Engine Object Flags bitmask (RF_Public, RF_Standalone, RF_Transient, RF_ClassDefaultObject, RF_AllFlags)"),
            ("EInternalObjectFlags", "EInternalObjectFlags.${1:Native}", "Unreal Engine Internal Object Flags bitmask (Native, Async, ReachableInCluster, RootSet)"),
            ("PropertyTypes", "PropertyTypes.${1:ObjectProperty}", "UE4SS Property Type identifiers (ObjectProperty, IntProperty, FloatProperty, StrProperty, BoolProperty, ArrayProperty, MapProperty, StructProperty)"),
            ("UnrealVersion", "UnrealVersion.GetMajor()", "UE4SS Engine Version Inspector (GetMajor(), GetMinor(), IsAtLeast(5, 1))"),
            ("UE4SS", "UE4SS.GetVersion()", "UE4SS Metadata Object (GetVersion() -> major, minor, hotfix)"),
            ("Mod", "Mod:SetSharedVariable(${1:\"Key\"}, ${2:Value})", "Mod Cross-Communication Interface (SetSharedVariable, GetSharedVariable)"),
            ("FVector", "FVector(${1:0.0}, ${2:0.0}, ${3:0.0})", "Unreal Engine 3D Vector constructor (X, Y, Z)"),
            ("FRotator", "FRotator(${1:0.0}, ${2:0.0}, ${3:0.0})", "Unreal Engine 3D Rotator constructor (Pitch, Yaw, Roll)"),
            ("FQuat", "FQuat(${1:0.0}, ${2:0.0}, ${3:0.0}, ${4:1.0})", "Unreal Engine Quaternion constructor (X, Y, Z, W)"),
            ("FTransform", "FTransform(${1:Rotator}, ${2:Translation}, ${3:Scale3D})", "Unreal Engine Transform constructor"),
            ("FLinearColor", "FLinearColor(${1:1.0}, ${2:1.0}, ${3:1.0}, ${4:1.0})", "Unreal Engine RGBA Linear Color constructor (R, G, B, A)"),
            ("FColor", "FColor(${1:255}, ${2:255}, ${3:255}, ${4:255})", "Unreal Engine 8-bit RGBA Color constructor"),
            ("FText", "FText(${1:\"Text\"})", "Unreal Engine Localized FText constructor"),
            ("FName", "FName(${1:\"Name\"})", "Unreal Engine FName identifier constructor"),
            ("FString", "FString(${1:\"String\"})", "Unreal Engine FString constructor"),
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
}

use palmodmanager_lib::commands::scanner::conflicts::extract_literal_hooks;
use palmodmanager_lib::commands::editor::{find_hook_start, extract_string_literal};

#[test]
fn test_extract_literal_hooks_direct_pcall_xpcall_and_multiline() {
    let lua_code = r#"
-- Direct call
RegisterHook("/Script/Pal.PalBullet:OnHitToActor", function(self) end)

-- Pcall reference
local ok, err = pcall(RegisterHook, "/Script/Pal.PalPlayerController:RequestUseItemToCharacter", function(self) end)

-- Single quotes
local ok2, err2 = pcall(RegisterHook, '/Script/Engine.Actor:K2_DestroyActor', callback)

-- NotifyOnNewObject via pcall
pcall(NotifyOnNewObject, "/Script/Pal.PalPlayerCharacter", function(self) end)

-- Xpcall
xpcall(RegisterHook, debug.traceback, "/Script/Pal.PalCharacter:Die", on_die)

-- Multiline pcall
pcall(
    RegisterHook,
    "/Script/Pal.PalDamageSubsystem:ApplyDamage",
    function(self) end
)

-- Commented out hook should not be extracted as active hook
-- RegisterHook("/Script/Pal.PalFake:CommentedOut", function() end)
"#;

    let hooks = extract_literal_hooks(lua_code);
    let targets: Vec<String> = hooks.iter().map(|(_, t, _, _)| t.clone()).collect();

    assert_eq!(targets.len(), 6, "Must extract exactly 6 active hooks");
    assert!(targets.contains(&"/Script/Pal.PalBullet:OnHitToActor".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalPlayerController:RequestUseItemToCharacter".to_string()));
    assert!(targets.contains(&"/Script/Engine.Actor:K2_DestroyActor".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalPlayerCharacter".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalCharacter:Die".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalDamageSubsystem:ApplyDamage".to_string()));
    assert!(!targets.contains(&"/Script/Pal.PalFake:CommentedOut".to_string()));

    println!("  [HOOKS] Extracted {} active hook declarations:", targets.len());
    for t in &targets {
        println!("          * Target: {}", t);
    }
}

#[test]
fn test_editor_syntax_validator_hook_start_and_literal_extraction() {
    // 1. Pcall with double quotes
    let line1 = r#"local ok, err = pcall(RegisterHook, "/Script/Pal.PalPlayerController:RequestUseItemToCharacter", function(self) end)"#;
    let start1 = find_hook_start(line1, 0).expect("Should find hook start");
    let (target1, _, _) = extract_string_literal(line1, start1).expect("Should extract string literal");
    assert_eq!(target1, "/Script/Pal.PalPlayerController:RequestUseItemToCharacter");

    // 2. Direct with single quotes
    let line2 = r#"RegisterHook('/Script/Engine.Actor:K2_DestroyActor', callback)"#;
    let start2 = find_hook_start(line2, 0).expect("Should find hook start");
    let (target2, _, _) = extract_string_literal(line2, start2).expect("Should extract string literal");
    assert_eq!(target2, "/Script/Engine.Actor:K2_DestroyActor");

    // 3. Xpcall with custom error handler
    let line3 = r#"xpcall(RegisterHook, debug.traceback, "/Script/Pal.PalCharacter:Die", on_die)"#;
    let start3 = find_hook_start(line3, 0).expect("Should find hook start");
    let (target3, _, _) = extract_string_literal(line3, start3).expect("Should extract string literal");
    assert_eq!(target3, "/Script/Pal.PalCharacter:Die");

    // 4. Line without hooks should return None
    let line4 = r#"local x = 42 + calculate_damage()"#;
    let start4 = find_hook_start(line4, 0);
    assert!(start4.is_none(), "Non-hook line should not have a hook start");

    println!("  [HOOKS] Monaco Editor Syntax Validator Literals:");
    println!("          * Pcall:  '{}'", target1);
    println!("          * Direct: '{}'", target2);
    println!("          * Xpcall: '{}'", target3);
}

#[test]
fn test_extract_literal_hooks_with_safe_register_hook_and_custom_wrappers() {
    let lua_code = r#"
-- PalWorldBedtimeExtended realistic snippet
local function safeRegisterHook(funcName, preCallback, postCallback)
    local ok, err = pcall(function()
        if postCallback then
            RegisterHook(funcName, preCallback, postCallback)
        else
            RegisterHook(funcName, preCallback)
        end
    end)
end

function Hooks.Init()
    -- Multiline safeRegisterHook 1
    safeRegisterHook(
        "/Game/Pal/Blueprint/Controller/Monster/BP_MonsterAIController_BaseCamp.BP_MonsterAIController_BaseCamp_C:InterruptSleepActivelyAction",
        function(self, Parameter) end
    )

    -- Multiline safeRegisterHook 2
    safeRegisterHook(
        "/Game/Pal/Blueprint/Controller/Monster/BP_MonsterAIController_BaseCamp.BP_MonsterAIController_BaseCamp_C:SetBaseCampActionSleep",
        function(self) end
    )

    -- Snake_case wrapper
    safe_register_hook('/Script/Pal.PalPlayerCharacter:OnJump', on_jump)

    -- PascalCase SafeRegisterHook
    SafeRegisterHook("/Script/Pal.PalUtility:SpawnPal", function() end)

    -- CamelCase direct registerHook
    registerHook("/Script/Engine.PlayerController:ClientRestart", restart)

    -- Wrapper for NotifyOnNewObject
    safeNotifyOnNewObject("/Script/Pal.PalMapObjectSpawner", function() end)
end
"#;

    let hooks = extract_literal_hooks(lua_code);
    let targets: Vec<String> = hooks.iter().map(|(_, t, _, _)| t.clone()).collect();
    let apis: Vec<String> = hooks.iter().map(|(api, _, _, _)| api.clone()).collect();

    // 1. Definition must NOT produce a false hook
    assert!(!targets.iter().any(|t| t.contains("funcName")), "Function definition argument must not be detected as hook");

    // 2. Both BedtimeExtended hooks must be accurately extracted
    assert!(targets.contains(&"/Game/Pal/Blueprint/Controller/Monster/BP_MonsterAIController_BaseCamp.BP_MonsterAIController_BaseCamp_C:InterruptSleepActivelyAction".to_string()));
    assert!(targets.contains(&"/Game/Pal/Blueprint/Controller/Monster/BP_MonsterAIController_BaseCamp.BP_MonsterAIController_BaseCamp_C:SetBaseCampActionSleep".to_string()));

    // 3. Other wrappers extracted
    assert!(targets.contains(&"/Script/Pal.PalPlayerCharacter:OnJump".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalUtility:SpawnPal".to_string()));
    assert!(targets.contains(&"/Script/Engine.PlayerController:ClientRestart".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalMapObjectSpawner".to_string()));

    assert_eq!(targets.len(), 6, "Must extract exactly 6 active hooks from wrappers and variants");

    // 4. APIs canonicalized
    assert_eq!(apis.iter().filter(|a| *a == "RegisterHook").count(), 5);
    assert_eq!(apis.iter().filter(|a| *a == "NotifyOnNewObject").count(), 1);

    println!("  [WRAPPER HOOKS] Successfully extracted {} hooks with canonical APIs:", hooks.len());
    for (api, target, line, code) in &hooks {
        println!("          * Line {}: [{}] {} ({})", line, api, target, code);
    }
}


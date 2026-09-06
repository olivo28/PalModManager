use palmodmanager_lib::commands::scanner::conflicts::extract_literal_hooks;
use palmodmanager_lib::commands::scanner::inspect_pak_file_tree;

#[test]
fn test_extract_literal_hooks_pcall_and_xpcall() {
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
"#;
    let hooks = extract_literal_hooks(lua_code);
    let targets: Vec<String> = hooks.into_iter().map(|(_, t, _, _)| t).collect();

    assert_eq!(targets.len(), 6);
    assert!(targets.contains(&"/Script/Pal.PalBullet:OnHitToActor".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalPlayerController:RequestUseItemToCharacter".to_string()));
    assert!(targets.contains(&"/Script/Engine.Actor:K2_DestroyActor".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalPlayerCharacter".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalCharacter:Die".to_string()));
    assert!(targets.contains(&"/Script/Pal.PalDamageSubsystem:ApplyDamage".to_string()));
}

#[test]
fn test_inspect_pak_from_archive_not_found() {
    let env = crate::helpers::TestEnv::new_steam_win64();
    let zip_path = crate::helpers::ZipBuilder::new("EmptyMod.zip")
        .add_text_file("dummy.txt", "hello")
        .build_in(&env.temp_dir);

    let res = inspect_pak_file_tree("NonExistent_P.pak".to_string(), Some(zip_path.to_str().unwrap().to_string()));
    assert!(res.is_err(), "Should return error when pak is not in archive");
    assert!(res.err().unwrap().contains("not found inside archive"));
}

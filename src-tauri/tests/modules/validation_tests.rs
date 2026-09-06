use palmodmanager_lib::commands::editor::{find_hook_start, extract_string_literal};

#[test]
fn test_find_hook_start_and_extract_string_literal() {
    let line1 = r#"local ok, err = pcall(RegisterHook, "/Script/Pal.PalPlayerController:RequestUseItemToCharacter", function(self) end)"#;
    let start1 = find_hook_start(line1, 0).expect("Should find hook start");
    let (target1, _, _) = extract_string_literal(line1, start1).expect("Should extract string literal");
    assert_eq!(target1, "/Script/Pal.PalPlayerController:RequestUseItemToCharacter");

    let line2 = r#"RegisterHook('/Script/Engine.Actor:K2_DestroyActor', callback)"#;
    let start2 = find_hook_start(line2, 0).expect("Should find hook start");
    let (target2, _, _) = extract_string_literal(line2, start2).expect("Should extract string literal");
    assert_eq!(target2, "/Script/Engine.Actor:K2_DestroyActor");

    let line3 = r#"xpcall(RegisterHook, debug.traceback, "/Script/Pal.PalCharacter:Die", on_die)"#;
    let start3 = find_hook_start(line3, 0).expect("Should find hook start");
    let (target3, _, _) = extract_string_literal(line3, start3).expect("Should extract string literal");
    assert_eq!(target3, "/Script/Pal.PalCharacter:Die");
}

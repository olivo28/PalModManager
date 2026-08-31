pub mod utils;
pub mod isolation;
pub mod actions;
pub mod core_lifecycle;
pub mod core;
pub mod export_import;

// Re-export public functions to keep existing client interfaces unbroken
pub use utils::{
    create_junction_or_symlink, remove_junction_or_symlink, sanitize_profile_id,
    get_profile_dir, copy_dir_all, move_path,
    save_pmm_meta,
};

pub use actions::{
    get_mod_folder_name, update_mods_txt_load_order, remove_from_mods_txt,
    disable_mod_internal, enable_mod_internal,
};

pub use core::{
    mod_matches_profile_entry,
    sync_current_profile_states, cleanup_profile_mod_lists,
    ensure_default_profile,
    auto_add_scanned_mods_to_profile,
};

pub use core_lifecycle::{
    set_profile_mod_state, switch_profile, create_profile, clone_profile,
    delete_profile, clear_profile, rename_profile,
};

pub use export_import::{
    export_profile_pack_internal, import_profile_pack_internal,
    ImportProfileResult,
};

use crate::models::AppData;

pub fn effective_force_ue4ss(data: &AppData) -> bool {
    let profile = data.profiles.iter().find(|p| p.id == data.current_profile_id);
    profile.and_then(|p| p.force_load_order_ue4ss)
        .or(data.settings.force_load_order_ue4ss)
        .unwrap_or(false)
}

pub fn effective_force_palschema(data: &AppData) -> bool {
    let profile = data.profiles.iter().find(|p| p.id == data.current_profile_id);
    profile.and_then(|p| p.force_load_order_palschema)
        .or(data.settings.force_load_order_palschema)
        .unwrap_or(false)
}

pub fn effective_hide_native_mods(data: &AppData) -> bool {
    let profile = data.profiles.iter().find(|p| p.id == data.current_profile_id);
    profile.and_then(|p| p.hide_native_mods)
        .or(data.settings.hide_native_mods)
        .unwrap_or(false)
}

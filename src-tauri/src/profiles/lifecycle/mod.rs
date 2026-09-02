// Profile lifecycle module — barrel re-exports all sub-modules
#![allow(unused_imports)]

pub mod backup_restore;
pub mod switch;
pub mod mod_state;
pub mod crud;

// Re-export backup & restore helpers
pub use backup_restore::{
    backup_game_files_to_profile,
    restore_profile_files_to_game,
    game_path_to_workshop_dir,
};

// Re-export switch
pub use switch::switch_profile;

// Re-export mod state
pub use mod_state::set_profile_mod_state;

// Re-export CRUD
pub use crud::{
    create_profile, clone_profile,
    delete_profile, clear_profile, rename_profile,
};

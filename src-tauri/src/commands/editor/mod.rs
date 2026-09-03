// Editor commands module — barrel re-exports all sub-modules
#![allow(unused_imports)]

pub mod types;
pub mod linter_lua;
pub mod linter_json;
pub mod validation;
pub mod completions;
pub mod workspace_index;
pub mod scaffolding;
pub mod sync_catalogs;

// Re-export types
pub use types::{EditorDiagnostic, EditorCompletion};
pub use workspace_index::*;
pub use scaffolding::*;
pub use sync_catalogs::{
    sync_blueprints_catalog, sync_datatables_catalog, sync_palschema_schemas,
    get_palschema_schemas_catalog, get_palschema_monaco_definitions,
    SyncCatalogResult,
};

// Re-export linters
pub use linter_lua::{lint_lua_syntax, LuaLexState};
pub use linter_json::{lint_json_syntax, find_key_position};

// Re-export Tauri commands
pub use validation::{validate_editor_code, scan_workspace_problems, find_hook_start, extract_string_literal};
pub use completions::{get_editor_completions, get_reflection_catalogs_status, ReflectionCatalogsStatus};


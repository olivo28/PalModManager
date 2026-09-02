use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorDiagnostic {
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub severity: String, // "error" | "warning" | "info"
    pub message: String,
    pub target: String,
    pub suggestion: Option<String>,
    pub category: String, // "ue4ss" | "palschema" | "syntax"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorCompletion {
    pub label: String,
    pub insert_text: String,
    pub kind: String, // "hook" | "class" | "function" | "delegate" | "table" | "struct" | "api" | "module"
    pub detail: Option<String>,
    pub documentation: Option<String>,
}

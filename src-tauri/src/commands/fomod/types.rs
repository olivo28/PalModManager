use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FomodInfo {
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub website: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FomodFileEntry {
    pub source: String,
    pub destination: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub is_folder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FomodFlag {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FomodFlagDependency {
    pub flag: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FomodVisibility {
    #[serde(default = "default_operator")]
    pub operator: String, // "And" | "Or"
    #[serde(default)]
    pub flag_dependencies: Vec<FomodFlagDependency>,
}

fn default_operator() -> String {
    "And".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FomodPlugin {
    pub id: String,
    pub name: String,
    pub description: String,
    pub image: Option<String>,
    pub image_base64: Option<String>,
    pub type_descriptor: String,
    pub files: Vec<FomodFileEntry>,
    pub condition_flags: Vec<FomodFlag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FomodGroup {
    pub name: String,
    pub group_type: String, // "SelectExactlyOne" | "SelectAtLeastOne" | "SelectAtMostOne" | "SelectAny" | "SelectAll"
    pub plugins: Vec<FomodPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FomodStep {
    pub name: String,
    pub visible: Option<FomodVisibility>,
    pub groups: Vec<FomodGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FomodPattern {
    pub dependencies: FomodVisibility,
    pub files: Vec<FomodFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FomodConfig {
    pub module_name: String,
    pub module_image: Option<String>,
    pub info: Option<FomodInfo>,
    pub required_install_files: Vec<FomodFileEntry>,
    pub install_steps: Vec<FomodStep>,
    pub conditional_file_installs: Vec<FomodPattern>,
    pub banner_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildFomodManifestPayload {
    pub zip_path: String,
    pub selected_files: Vec<FomodFileEntry>,
    pub custom_name: Option<String>,
    pub custom_folder: Option<String>,
    pub mod_name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub fomod_choices: Option<std::collections::HashMap<String, Vec<String>>>,
}

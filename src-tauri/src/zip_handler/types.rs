#[derive(Debug, Clone)]
pub enum DetectedModType {
    Ue4ss,
    PalSchema,
    Pak,
    LogicMods,
    Hybrid,
    Altermatic,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ZipAnalysis {
    pub detected_type: DetectedModType,
    pub has_lua: bool,
    pub has_json: bool,
    pub has_palschema_json: bool,
    pub has_pak: bool,
    pub has_altermatic: bool,
    #[allow(dead_code)]
    pub has_dll: bool,
    pub has_info_json: bool,
    pub pak_destination_hint: Option<String>,
    pub root_folder: Option<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    SevenZip,
    Rar,
    RawPak,
}

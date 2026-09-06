use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Hermetic temporary game environment for multi-platform integration tests.
pub struct TestEnv {
    pub temp_dir: PathBuf,
    pub game_root: PathBuf,
    pub program_data: PathBuf,
}

impl TestEnv {
    /// Creates a simulated Steam Win64 Palworld environment.
    pub fn new_steam_win64() -> Self {
        let temp_dir = std::env::temp_dir().join(format!("pmm_test_steam_{}", uuid::Uuid::new_v4()));
        let game_root = temp_dir.join("Palworld");

        // Pal/Binaries/Win64
        let win64_dir = game_root.join("Pal").join("Binaries").join("Win64");
        fs::create_dir_all(&win64_dir).unwrap();
        fs::write(win64_dir.join("Palworld-Win64-Shipping.exe"), b"MOCK_EXE").unwrap();
        fs::write(win64_dir.join("dwmapi.dll"), b"MOCK_DLL").unwrap();

        // Pal/Binaries/Win64/ue4ss/Mods/PalSchema/mods
        let ue4ss_mods = win64_dir.join("ue4ss").join("Mods");
        let palschema_mods = ue4ss_mods.join("PalSchema").join("mods");
        fs::create_dir_all(&palschema_mods).unwrap();

        // Pal/Content/Paks/~mods and LogicMods
        let paks_dir = game_root.join("Pal").join("Content").join("Paks");
        fs::create_dir_all(paks_dir.join("~mods")).unwrap();
        fs::create_dir_all(paks_dir.join("LogicMods")).unwrap();
        fs::write(paks_dir.join("Pal-Windows.pak"), b"MOCK_PAK").unwrap();

        // Profiles / configs
        let program_data = temp_dir.join("PalModManagerData");
        let profile_dir = program_data.join("profiles").join("default").join("archived_configs");
        fs::create_dir_all(&profile_dir).unwrap();

        Self { temp_dir, game_root, program_data }
    }

    /// Creates a simulated Xbox Game Pass (WinGDK) Palworld environment.
    pub fn new_xbox_wingdk() -> Self {
        let temp_dir = std::env::temp_dir().join(format!("pmm_test_xbox_{}", uuid::Uuid::new_v4()));
        let game_root = temp_dir.join("Palworld");

        // Pal/Binaries/WinGDK
        let wingdk_dir = game_root.join("Pal").join("Binaries").join("WinGDK");
        fs::create_dir_all(&wingdk_dir).unwrap();
        fs::write(wingdk_dir.join("Palworld-WinGDK-Shipping.exe"), b"MOCK_EXE").unwrap();
        fs::write(wingdk_dir.join("xinput1_3.dll"), b"MOCK_DLL").unwrap();

        // Pal/Binaries/WinGDK/ue4ss/Mods
        let ue4ss_mods = wingdk_dir.join("ue4ss").join("Mods");
        fs::create_dir_all(&ue4ss_mods).unwrap();

        // Pal/Content/Paks/~mods and LogicMods
        let paks_dir = game_root.join("Pal").join("Content").join("Paks");
        fs::create_dir_all(paks_dir.join("~mods")).unwrap();
        fs::create_dir_all(paks_dir.join("LogicMods")).unwrap();
        fs::write(paks_dir.join("Pal-Windows.pak"), b"MOCK_PAK").unwrap();

        // Profiles / configs
        let program_data = temp_dir.join("PalModManagerData");
        let profile_dir = program_data.join("profiles").join("default").join("archived_configs");
        fs::create_dir_all(&profile_dir).unwrap();

        Self { temp_dir, game_root, program_data }
    }

    /// Creates a simulated Steam Workshop environment with PalModSettings.ini.
    pub fn new_workshop() -> Self {
        let temp_dir = std::env::temp_dir().join(format!("pmm_test_ws_{}", uuid::Uuid::new_v4()));
        let game_root = temp_dir.join("Palworld");

        // Base executable and Content
        let win64_dir = game_root.join("Pal").join("Binaries").join("Win64");
        fs::create_dir_all(&win64_dir).unwrap();
        fs::write(win64_dir.join("Palworld-Win64-Shipping.exe"), b"MOCK_EXE").unwrap();

        let paks_dir = game_root.join("Pal").join("Content").join("Paks").join("~mods");
        fs::create_dir_all(&paks_dir).unwrap();

        // Mods root with PalModSettings.ini
        let mods_root = game_root.join("Mods");
        fs::create_dir_all(&mods_root).unwrap();
        let ini_content = "[PalModSettings]\r\nbGlobalEnableMod=True\r\nActiveModList=UE4SS\r\n";
        fs::write(mods_root.join("PalModSettings.ini"), ini_content).unwrap();

        // NativeMods/UE4SS/Mods and ManagedMods/PalSchema
        let native_ue4ss = mods_root.join("NativeMods").join("UE4SS").join("Mods");
        fs::create_dir_all(&native_ue4ss).unwrap();
        let managed_palschema = mods_root.join("ManagedMods").join("PalSchema");
        fs::create_dir_all(&managed_palschema).unwrap();

        // Profiles / configs
        let program_data = temp_dir.join("PalModManagerData");
        let profile_dir = program_data.join("profiles").join("default").join("archived_configs");
        fs::create_dir_all(&profile_dir).unwrap();

        Self { temp_dir, game_root, program_data }
    }

    /// Installs a mod ZIP directly into this test game environment using the real analysis & installer pipeline.
    pub fn install_zip(&self, zip_path: &Path) -> Result<palmodmanager_lib::models::ModInfo, String> {
        let zip_str = zip_path.to_str().ok_or_else(|| "Invalid zip path".to_string())?;
        let extract_target = std::env::temp_dir().join(format!("pmm_extract_{}", uuid::Uuid::new_v4()));
        let extracted_dir = palmodmanager_lib::zip_handler::extract_zip_to_temp(zip_str, &extract_target)?;
        let analysis = palmodmanager_lib::zip_handler::analyze_zip(zip_str)?;
        let zip_filename = zip_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let nexus_mod_id = palmodmanager_lib::zip_handler::extract_nexus_id_from_path(&zip_filename);

        let res = palmodmanager_lib::installer::install_mod(
            &self.game_root.to_string_lossy(),
            &extracted_dir,
            &analysis,
            &zip_filename,
            nexus_mod_id,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            false,
            false,
        );
        let _ = fs::remove_dir_all(&extract_target);
        res
    }

    /// Builds a ZIP from a `ZipBuilder` inside this test environment's temp directory and installs it via the real pipeline.
    pub fn install_builder(&self, builder: &ZipBuilder) -> Result<palmodmanager_lib::models::ModInfo, String> {
        let zip_path = builder.build_in(&self.temp_dir);
        self.install_zip(&zip_path)
    }

    /// Scans the test game environment using the real full-disk scanner engine.
    pub fn scan_mods(&self) -> Vec<palmodmanager_lib::models::ModInfo> {
        self.scan_mods_with_db(&[])
    }

    /// Scans the test game environment merging against existing database mods (mirroring production scanMods).
    pub fn scan_mods_with_db(&self, db_mods: &[palmodmanager_lib::models::ModInfo]) -> Vec<palmodmanager_lib::models::ModInfo> {
        palmodmanager_lib::commands::mod_commands::scan_mods_internal(
            &self.game_root.to_string_lossy(),
            &self.program_data.to_string_lossy(),
            "default",
            &[],
            db_mods,
        )
    }

    /// Archives configurations for a given mod into the profile storage.
    pub fn archive_configs(&self, mod_info: &palmodmanager_lib::models::ModInfo) -> Result<usize, String> {
        palmodmanager_lib::commands::config_archive::archive_mod_configs(
            mod_info,
            &self.program_data.to_string_lossy(),
            "default",
            &self.game_root.to_string_lossy(),
        )
    }

    /// Performs complete uninstallation of a mod on disk, mimicking production remove_mod:
    /// cleans up game_path, extra_files, sidecars, and shared/ mod folders.
    pub fn remove_mod_on_disk(&self, mod_info: &palmodmanager_lib::models::ModInfo) {
        let delete_path_and_sidecar = |path_str: &str| {
            if path_str.is_empty() { return; }
            let p = Path::new(path_str);
            if p.exists() {
                if p.is_dir() {
                    let _ = fs::remove_dir_all(p);
                } else {
                    let _ = fs::remove_file(p);
                    let sidecar = PathBuf::from(format!("{}.pmm.json", path_str));
                    if sidecar.exists() {
                        let _ = fs::remove_file(sidecar);
                    }
                }
            }
        };

        delete_path_and_sidecar(&mod_info.game_path);
        delete_path_and_sidecar(&mod_info.disabled_path);
        for extra in &mod_info.extra_files {
            delete_path_and_sidecar(extra);
        }

        let folder_name = palmodmanager_lib::profiles::get_mod_folder_name(mod_info);
        let binaries_dir = palmodmanager_lib::dependency_checker::get_binaries_dir(&self.game_root);
        let ue4ss_roots = vec![
            binaries_dir.join("ue4ss").join("Mods"),
            binaries_dir.join("Mods"),
            self.game_root.join("Mods").join("NativeMods").join("UE4SS").join("Mods"),
        ];

        for u_dir in &ue4ss_roots {
            if !u_dir.exists() { continue; }
            let target_mod_folder = u_dir.join(&folder_name);
            if target_mod_folder.exists() {
                let _ = fs::remove_dir_all(&target_mod_folder);
            }
            let target_mod_name = u_dir.join(&mod_info.name);
            if target_mod_name.exists() {
                let _ = fs::remove_dir_all(&target_mod_name);
            }

            let shared_dir = u_dir.join("shared");
            if shared_dir.exists() {
                let s_mod = shared_dir.join(&folder_name);
                if s_mod.exists() {
                    let _ = fs::remove_dir_all(&s_mod);
                }
                let s_name = shared_dir.join(&mod_info.name);
                if s_name.exists() {
                    let _ = fs::remove_dir_all(&s_name);
                }
            }
        }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

/// Fluent builder for creating valid, in-memory or on-disk ZIP archives.
pub struct ZipBuilder {
    pub filename: String,
    pub files: Vec<(String, Vec<u8>)>,
}

impl ZipBuilder {
    pub fn new(filename: &str) -> Self {
        Self {
            filename: filename.to_string(),
            files: Vec::new(),
        }
    }

    pub fn add_file(mut self, path_in_zip: &str, content: &[u8]) -> Self {
        self.files.push((path_in_zip.to_string(), content.to_vec()));
        self
    }

    pub fn add_text_file(self, path_in_zip: &str, text: &str) -> Self {
        self.add_file(path_in_zip, text.as_bytes())
    }

    pub fn build_in(&self, dir: &Path) -> PathBuf {
        let out_path = dir.join(&self.filename);
        let file = fs::File::create(&out_path).unwrap();
        let mut zip = zip::write::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for (path, data) in &self.files {
            zip.start_file(path, options).unwrap();
            zip.write_all(data).unwrap();
        }

        zip.finish().unwrap();
        out_path
    }
}

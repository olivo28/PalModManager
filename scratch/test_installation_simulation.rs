use std::fs;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

// --- SIMULATED LOGIC FROM INSTALLER ---

fn sanitize_folder_name(name: &str) -> String {
    let mut cleaned = name.replace(|c: char| {
        c == '/' || c == '\\' || c == ':' || c == '*' || c == '?' || c == '"' || c == '<' || c == '>' || c == '|'
    }, "");
    cleaned = cleaned.trim().to_string();
    cleaned
}

fn clean_zip_name(zip_name: &str) -> String {
    let clean = zip_name.trim_end_matches(".zip").trim_end_matches(".rar");
    sanitize_folder_name(clean)
}

fn simulate_hybrid_installation(
    zip_path: &Path,
    mock_game_root: &Path,
    nexus_mod_id: Option<u32>,
) -> Result<(), String> {
    let file_name = zip_path.file_name().unwrap().to_string_lossy().to_string();
    println!("\n>>> Simulando instalación híbrida para: {}", file_name);

    let clean_stem = clean_zip_name(&file_name);
    let mod_name = clean_stem.clone(); // Simulation defaults to clean zip filename
    let safe_folder_name = sanitize_folder_name(&mod_name);


    // 1. Abrir archivo ZIP
    let file = fs::File::open(zip_path).map_err(|e| format!("Cannot open zip: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid zip: {}", e))?;

    // Obtener y listar todas las rutas internas
    let mut files = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| format!("Cannot read entry: {}", e))?;
        files.push(entry.name().to_string());
    }
    
    println!("  Estructura de archivos dentro del ZIP:");
    for f in &files {
        println!("    - {}", f);
    }

    // 2. Definir rutas en el juego simulado (Xbox / WinGDK por defecto o Steam si detecta)
    let binaries_dir = mock_game_root.join("Pal").join("Binaries").join("WinGDK");
    let paks_dir = mock_game_root.join("Pal").join("Content").join("Paks").join("~mods");
    
    let is_palschema_hybrid = files.iter().any(|f| f.to_lowercase().contains("palschema"));
    let ue4ss_mods_dir = if is_palschema_hybrid {
        binaries_dir.join("ue4ss").join("Mods").join("PalSchema").join("mods").join(&safe_folder_name)
    } else {
        binaries_dir.join("ue4ss").join("Mods").join(&safe_folder_name)
    };

    println!("Rutas Destino Simuladas:");
    println!("  - Directorio Paks: {}", paks_dir.display());
    println!("  - Directorio Scripts: {}", ue4ss_mods_dir.display());

    // 3. Iterar archivos del ZIP y simular la copia
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| format!("Cannot read entry: {}", e))?;
        let entry_name = entry.name();
        
        if entry_name.ends_with('/') {
            continue;
        }

        let lower = entry_name.to_lowercase();
        let file_basename = Path::new(entry_name).file_name().unwrap().to_string_lossy().to_string();

        // Skip copying the "Pal" folder (which holds Paks) or raw pak files into the scripts folder
        let is_pak_content = lower.contains("pal/content/paks") || lower.ends_with(".pak") || lower.ends_with(".ucas") || lower.ends_with(".utoc");

        if lower.ends_with(".pak") {
            let target_file = paks_dir.join(&file_basename);
            println!("  [COPIA] Archivo .pak -> {}", target_file.display());
        } else if !is_pak_content {
            if lower.ends_with(".lua") {
                let target_file = ue4ss_mods_dir.join("Scripts").join(&file_basename);
                println!("  [COPIA] Script .lua  -> {}", target_file.display());
            } else if lower.ends_with(".json") || lower.ends_with(".jsonc") || lower.ends_with(".txt") {
                let target_file = ue4ss_mods_dir.join(&file_basename);
                println!("  [COPIA] Config/Info  -> {}", target_file.display());
            }
        }
    }


    let id = if let Some(n) = nexus_mod_id {
        format!("{}-{}", n, safe_folder_name)
    } else {
        safe_folder_name.clone()
    };
    println!("  [REGISTRO] Mod registrado con ID: {}", id);

    Ok(())
}

fn main() {
    println!("==================================================");
    println!("     INSTALACIÓN HÍBRIDA DRY-RUN SIMULATION       ");
    println!("==================================================");

    // Definimos el juego simulado en la carpeta scratch
    let scratch_dir = PathBuf::from("scratch");
    let mock_game = scratch_dir.join("mock_game_environment");
    
    // Limpiamos el entorno anterior
    let _ = fs::remove_dir_all(&mock_game);
    let _ = fs::create_dir_all(&mock_game);

    let downloads_dir = Path::new("C:\\Users\\Antikux\\Downloads");
    
    let zip1 = downloads_dir.join("PalVariety DexEdition V0.0.16 4140 1 2026-07-27T23-54Z qJmcAd9Nd.zip");
    let zip2 = downloads_dir.join("PalVariety Rarity Shiny4096 V1.1.3 4140 4 2026-07-27T23-52Z QXTyhgia8.zip");

    if zip1.exists() {
        if let Err(e) = simulate_hybrid_installation(&zip1, &mock_game, Some(4140)) {
            println!("Error simulando Zip 1: {}", e);
        }
    } else {
        println!("Zip 1 no encontrado en Downloads.");
    }

    if zip2.exists() {
        if let Err(e) = simulate_hybrid_installation(&zip2, &mock_game, Some(4140)) {
            println!("Error simulando Zip 2: {}", e);
        }
    } else {
        println!("Zip 2 no encontrado en Downloads.");
    }

    println!("\n==================================================");
    println!("          SIMULACIÓN DE INSTALACIÓN COMPLETA      ");
    println!("==================================================");
}

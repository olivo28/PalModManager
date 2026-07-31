use palmodmanager_lib::db;
use palmodmanager_lib::nexus;
use tauri::async_runtime::block_on;
use std::path::PathBuf;

fn main() {
    println!("==================================================");
    println!("      REAL API TEST: ProgressiveWeaponProficiency");
    println!("==================================================\n");

    let local_app_data = std::env::var("LOCALAPPDATA").expect("LOCALAPPDATA environment variable not found");
    let program_path = PathBuf::from(local_app_data).join("PalModManager");
    println!("1. Loading settings from DB at: {}", program_path.display());
    
    let data = db::load_db(&program_path.to_string_lossy());
    let api_key = data.settings.nexus_api_key.clone();
    
    let has_key = api_key.as_ref().map_or(false, |k| !k.is_empty());
    if has_key {
        println!("[OK] Found Nexus API Key in settings.");
    } else {
        println!("[NOTE] No Nexus API Key was found. Querying GraphQL v2 anonymously (REST v1 files query will be skipped).");
    }

    let mod_id = 3787;

    block_on(async {
        if has_key {
            let key = api_key.clone().unwrap();
            println!("\n2. Executing LIVE REST v1 files.json API Query for Mod ID: {}...", mod_id);
            let url = format!("https://api.nexusmods.com/v1/games/palworld/mods/{}/files.json", mod_id);
            let client = reqwest::Client::builder()
                .user_agent("PalModManager/1.0.0 (Tauri App)")
                .build()
                .unwrap();
                
            let resp = client.get(&url)
                .header("apikey", &key)
                .send()
                .await;
                
            match resp {
                Ok(r) => {
                    if r.status().is_success() {
                        let json_val: serde_json::Value = r.json().await.unwrap();
                        println!("\n>>> LIVE REST v1 FILES RESPONSE:");
                        println!("{}", serde_json::to_string_pretty(&json_val).unwrap());
                    } else {
                        println!("\n[ERROR] REST v1 API returned HTTP status: {}", r.status());
                    }
                }
                Err(e) => {
                    println!("\n[ERROR] Network request failed: {}", e);
                }
            }
        } else {
            println!("\n2. [SKIPPED] REST v1 files.json query skipped (requires API Key).");
        }
        
        println!("\n3. Executing LIVE GraphQL v2 query (Anonymous)...");
        match nexus::fetch_mod_info(mod_id, api_key.as_ref().map(|k| k.as_str())).await {
            Ok(info) => {
                println!("\n>>> LIVE GraphQL v2 RESPONSE SUMMARY:");
                println!("--------------------------------------------------");
                println!("Name:         {}", info.name);
                println!("Author:       {}", info.author);
                println!("Summary:      {}", info.summary);
                println!("Version:      {}", info.version);
                println!("Downloads:    {}", info.downloads);
                println!("Endorsements: {}", info.endorsements);
                println!("Picture URL:  {}", info.picture_url);
                println!("Category:     {}", info.category);
                println!("Tags:         {:?}", info.tags);
                println!("--------------------------------------------------");
            }
            Err(e) => {
                println!("\n[ERROR] GraphQL v2 query failed: {}", e);
            }
        }
    });

    println!("\n==================================================");
}

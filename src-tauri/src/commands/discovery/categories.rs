use reqwest::StatusCode;
use serde::Deserialize;
use tauri::State;
use crate::state::AppState;
use super::client::{get_client, REST_BASE_URL};
use super::types::DiscoveryCategory;

/// Query categories for Palworld
#[tauri::command]
pub async fn get_discovery_categories(state: State<'_, AppState>) -> Result<Vec<DiscoveryCategory>, String> {
    let token = crate::nexus_oauth::ensure_valid_nexus_token(&state).await;

    let client = get_client(token.as_deref());
    let url = format!("{}/games/palworld/categories.json", REST_BASE_URL);
    let mut resp = client.get(&url).send().await;

    if let Ok(ref r) = resp {
        if r.status() == StatusCode::UNAUTHORIZED || r.status().as_u16() == 402 {
            let anon_client = get_client(None);
            resp = anon_client.get(&url).send().await;
        }
    }

    let resp = resp.map_err(|e| format!("Failed to fetch categories: {}", e))?;

    let full_default_categories = vec![
        DiscoveryCategory { category_id: 1, name: "Animations".to_string() },
        DiscoveryCategory { category_id: 2, name: "Audio".to_string() },
        DiscoveryCategory { category_id: 3, name: "Characters".to_string() },
        DiscoveryCategory { category_id: 4, name: "Gameplay".to_string() },
        DiscoveryCategory { category_id: 5, name: "Miscellaneous".to_string() },
        DiscoveryCategory { category_id: 6, name: "Outfits".to_string() },
        DiscoveryCategory { category_id: 7, name: "Pals".to_string() },
        DiscoveryCategory { category_id: 8, name: "Palworld".to_string() },
        DiscoveryCategory { category_id: 9, name: "Scripts".to_string() },
        DiscoveryCategory { category_id: 10, name: "User Interface".to_string() },
        DiscoveryCategory { category_id: 11, name: "Utilities".to_string() },
        DiscoveryCategory { category_id: 12, name: "Visuals".to_string() },
        DiscoveryCategory { category_id: 13, name: "Weapons".to_string() },
    ];

    if !resp.status().is_success() {
        return Ok(full_default_categories);
    }

    #[derive(Deserialize)]
    struct RestCategoryItem {
        category_id: u32,
        name: String,
    }

    let items: Vec<RestCategoryItem> = resp.json().await.unwrap_or_default();
    if items.is_empty() {
        return Ok(full_default_categories);
    }

    let categories = items
        .into_iter()
        .map(|c| DiscoveryCategory {
            category_id: c.category_id,
            name: c.name,
        })
        .collect();

    Ok(categories)
}

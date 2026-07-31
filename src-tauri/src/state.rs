use std::sync::Mutex;
use crate::models::AppData;

pub struct AppState {
    pub data: Mutex<AppData>,
}

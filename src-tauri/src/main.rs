// Prevents initial console window on Windows unless dynamically allocated
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    palmodmanager_lib::run();
}

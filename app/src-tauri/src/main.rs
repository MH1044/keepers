//! The Keepers desktop host. It only marshals: every command is a thin call into the core, and
//! the window never computes a score of its own.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::app_info])
        .run(tauri::generate_context!())
        .expect("Keepers could not open its window");
}

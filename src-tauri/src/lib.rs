#![allow(dead_code)]

mod engine;
mod input;
mod network;
mod coordinator;
mod commands;

use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = commands::AppState {
        handle: Arc::new(Mutex::new(None)),
        connect_tx: Arc::new(Mutex::new(None)),
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::start_server,
            commands::start_client,
            commands::stop_coordinator,
            commands::connect_to_peer,
            commands::request_accessibility_permission,
        ])
        .run(tauri::generate_context!())
        .expect("error while running flowcontrol");
}

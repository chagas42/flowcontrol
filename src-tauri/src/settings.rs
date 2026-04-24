//! Persistent user settings. Wraps `tauri-plugin-store` with a typed
//! `Settings` struct so callers don't have to JSON-sniff keys.
//!
//! Defaults are conservative: telemetry off, no autostart, system theme,
//! standard mDNS port.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

const STORE_PATH: &str = "settings.json";
const KEY: &str = "settings";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub launch_on_login: bool,
    pub auto_reconnect: bool,
    pub telemetry: bool,
    pub theme: String, // "system" | "light" | "dark"
    pub mdns_port: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            launch_on_login: false,
            auto_reconnect: true,
            telemetry: false,
            theme: "system".to_string(),
            mdns_port: 7070,
        }
    }
}

fn store<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<std::sync::Arc<tauri_plugin_store::Store<R>>, tauri_plugin_store::Error> {
    app.store(STORE_PATH)
}

pub fn load<R: Runtime>(app: &AppHandle<R>) -> Settings {
    let Ok(store) = store(app) else {
        return Settings::default();
    };
    store
        .get(KEY)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

pub fn save<R: Runtime>(app: &AppHandle<R>, s: &Settings) -> Result<(), String> {
    let store = store(app).map_err(|e| e.to_string())?;
    let value = serde_json::to_value(s).map_err(|e| e.to_string())?;
    store.set(KEY, value);
    store.save().map_err(|e| e.to_string())
}

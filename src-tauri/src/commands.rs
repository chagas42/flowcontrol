use serde::Deserialize;
use std::sync::Arc;
use tauri::{Emitter, State};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[cfg(target_os = "macos")]
use crate::input::macos::MacOSCapture;
#[cfg(target_os = "macos")]
use crate::input::{InputCapture, PermissionStatus};

use crate::coordinator::Coordinator;
use crate::engine::screen_layout::{NeighborSide, ScreenDimensions};
use crate::settings::{self, Settings};

// Store the JoinHandle so we can abort the spawned event loop.
// The Coordinator's Drop impl will ensure cleanup (stop capture, stop network).
pub struct AppState {
    pub handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    pub connect_tx: Arc<Mutex<Option<tokio::sync::mpsc::Sender<String>>>>,
    pub pair_response_tx: Arc<Mutex<Option<tokio::sync::mpsc::Sender<bool>>>>,
    pub pause_tx: Arc<Mutex<Option<tokio::sync::mpsc::Sender<bool>>>>,
}

#[derive(Deserialize)]
pub enum SideParam {
    Left,
    Right,
    Top,
    Bottom,
}

impl From<SideParam> for NeighborSide {
    fn from(val: SideParam) -> Self {
        match val {
            SideParam::Left => NeighborSide::Left,
            SideParam::Right => NeighborSide::Right,
            SideParam::Top => NeighborSide::Top,
            SideParam::Bottom => NeighborSide::Bottom,
        }
    }
}

#[tauri::command]
pub async fn start_server(
    name: String,
    width: u32,
    height: u32,
    side: SideParam,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut lock = state.handle.lock().await;

    if let Some(old) = lock.take() {
        old.abort();
    }
    *state.connect_tx.lock().await = None;
    *state.pair_response_tx.lock().await = None;
    *state.pause_tx.lock().await = None;

    let (connect_tx, connect_rx) = tokio::sync::mpsc::channel(32);
    let (pair_tx, pair_rx) = tokio::sync::mpsc::channel(4);
    let (pause_tx, pause_rx) = tokio::sync::mpsc::channel(4);
    let name_clone = name.clone();
    let mut coordinator = Coordinator::new_server(
        name_clone,
        ScreenDimensions { width, height },
        side.into(),
        Some(app_handle),
        connect_rx,
        pair_rx,
        pause_rx,
    );

    let handle = tokio::spawn(async move {
        // Run until aborted
        let _ = coordinator.run_as_server(&name).await;
    });

    *lock = Some(handle);
    *state.connect_tx.lock().await = Some(connect_tx);
    *state.pair_response_tx.lock().await = Some(pair_tx);
    *state.pause_tx.lock().await = Some(pause_tx);
    Ok(())
}

#[tauri::command]
pub async fn start_client(
    width: u32,
    height: u32,
    side: SideParam,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut lock = state.handle.lock().await;

    if let Some(old) = lock.take() {
        old.abort();
    }
    *state.connect_tx.lock().await = None;
    *state.pair_response_tx.lock().await = None;
    *state.pause_tx.lock().await = None;

    let (connect_tx, connect_rx) = tokio::sync::mpsc::channel(32);
    let (pair_tx, pair_rx) = tokio::sync::mpsc::channel(4);
    let (pause_tx, pause_rx) = tokio::sync::mpsc::channel(4);
    let mut coordinator = Coordinator::new_client(
        default_client_name(),
        ScreenDimensions { width, height },
        side.into(),
        Some(app_handle.clone()),
        connect_rx,
        pair_rx,
        pause_rx,
    );

    let handle = tokio::spawn(async move {
        let _ = coordinator.run_as_client().await;
    });

    *lock = Some(handle);
    *state.connect_tx.lock().await = Some(connect_tx);
    *state.pair_response_tx.lock().await = Some(pair_tx);
    *state.pause_tx.lock().await = Some(pause_tx);

    // Injection via CGEventPost requires Accessibility on the client too.
    // (No event tap needed, but posting to other apps is still gated by TCC.)
    #[cfg(target_os = "macos")]
    {
        if MacOSCapture::new().permission_status() != PermissionStatus::Granted {
            let _ = app_handle.emit("permission-required", ());
        }
    }

    Ok(())
}

fn default_client_name() -> String {
    std::env::var("FLOWCONTROL_DEVICE_NAME").unwrap_or_else(|_| {
        if cfg!(target_os = "macos") {
            "This Mac".to_string()
        } else if cfg!(target_os = "windows") {
            "This PC".to_string()
        } else {
            "FlowControl".to_string()
        }
    })
}

#[tauri::command]
pub async fn stop_coordinator(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut lock = state.handle.lock().await;
    if let Some(handle) = lock.take() {
        handle.abort();
    }
    *state.connect_tx.lock().await = None;
    *state.pair_response_tx.lock().await = None;
    *state.pause_tx.lock().await = None;
    let _ = app_handle.emit("status-changed", "Stopped");
    Ok(())
}

/// Checks Accessibility status silently — no dialog, no prompt.
/// Returns true if already granted.
#[tauri::command]
pub async fn check_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        MacOSCapture::new().permission_status() == PermissionStatus::Granted
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Opens System Settings → Accessibility (one-shot prompt).
/// Call this ONCE when the user clicks the button — then poll
/// check_accessibility_permission to detect when they toggle it on.
#[tauri::command]
pub async fn request_accessibility_permission() {
    #[cfg(target_os = "macos")]
    {
        MacOSCapture::new().request_permission();
    }
}

#[tauri::command]
pub async fn connect_to_peer(peer_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let tx = state.connect_tx.lock().await.clone();
    if let Some(tx) = tx {
        tx.send(peer_id).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Coordinator not running".into())
    }
}

#[tauri::command]
pub async fn pair_accept(state: State<'_, AppState>) -> Result<(), String> {
    send_pair_response(&state, true).await
}

#[tauri::command]
pub async fn pair_decline(state: State<'_, AppState>) -> Result<(), String> {
    send_pair_response(&state, false).await
}

async fn send_pair_response(state: &State<'_, AppState>, accept: bool) -> Result<(), String> {
    let tx = state.pair_response_tx.lock().await.clone();
    if let Some(tx) = tx {
        tx.send(accept).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Coordinator not running".into())
    }
}

#[tauri::command]
pub async fn pause_sharing(state: State<'_, AppState>) -> Result<(), String> {
    send_pause(&state, true).await
}

#[tauri::command]
pub async fn resume_sharing(state: State<'_, AppState>) -> Result<(), String> {
    send_pause(&state, false).await
}

async fn send_pause(state: &State<'_, AppState>, paused: bool) -> Result<(), String> {
    let tx = state.pause_tx.lock().await.clone();
    if let Some(tx) = tx {
        tx.send(paused).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Coordinator not running".into())
    }
}

#[tauri::command]
pub async fn get_settings(app_handle: tauri::AppHandle) -> Result<Settings, String> {
    Ok(settings::load(&app_handle))
}

#[tauri::command]
pub async fn set_settings(new: Settings, app_handle: tauri::AppHandle) -> Result<(), String> {
    let old = settings::load(&app_handle);
    settings::save(&app_handle, &new)?;
    // TODO: if old.mdns_port != new.mdns_port and a coordinator is running,
    // tear down the advertising/browsing task and re-advertise on the new
    // port. The network layer currently hard-codes PORT=7878 for TCP and the
    // mDNS discovery port is separate — flag on the follow-up pass.
    // TODO: if !old.auto_reconnect && new.auto_reconnect and we have a
    // last-peer on disk, kick off connect_to_peer(last_peer_id) here.
    // TODO: integrate tauri-plugin-autostart for launch_on_login.
    let _ = old;
    Ok(())
}

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use serde::Deserialize;
use tauri::State;

#[cfg(target_os = "macos")]
use crate::input::{InputCapture, PermissionStatus};
#[cfg(target_os = "macos")]
use crate::input::macos::MacOSCapture;

use crate::coordinator::Coordinator;
use crate::engine::screen_layout::{NeighborSide, ScreenDimensions};

// Store the JoinHandle so we can abort the spawned event loop.
// The Coordinator's Drop impl will ensure cleanup (stop capture, stop network).
pub struct AppState {
    pub handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    pub connect_tx: Arc<Mutex<Option<tokio::sync::mpsc::Sender<String>>>>,
}

#[derive(Deserialize)]
pub enum SideParam {
    Left,
    Right,
    Top,
    Bottom,
}

impl Into<NeighborSide> for SideParam {
    fn into(self) -> NeighborSide {
        match self {
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

    let (tx, rx) = tokio::sync::mpsc::channel(32);
    let mut coordinator = Coordinator::new_server(ScreenDimensions { width, height }, side.into(), Some(app_handle), rx);
    
    let handle = tokio::spawn(async move {
        // Run until aborted
        let _ = coordinator.run_as_server(&name).await;
    });

    *lock = Some(handle);
    *state.connect_tx.lock().await = Some(tx);
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

    let (tx, rx) = tokio::sync::mpsc::channel(32);
    let mut coordinator = Coordinator::new_client(ScreenDimensions { width, height }, side.into(), Some(app_handle), rx);

    let handle = tokio::spawn(async move {
        let _ = coordinator.run_as_client().await;
    });

    *lock = Some(handle);
    *state.connect_tx.lock().await = Some(tx);
    Ok(())
}

#[tauri::command]
pub async fn stop_coordinator(state: State<'_, AppState>) -> Result<(), String> {
    let mut lock = state.handle.lock().await;
    if let Some(handle) = lock.take() {
        handle.abort();
    }
    *state.connect_tx.lock().await = None;
    Ok(())
}

/// Checks Accessibility status silently — no dialog, no prompt.
/// Returns true if already granted.
#[tauri::command]
pub async fn check_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        return MacOSCapture::new().permission_status() == PermissionStatus::Granted;
    }
    #[cfg(not(target_os = "macos"))]
    true
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
pub async fn connect_to_peer(
    peer_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let tx = state.connect_tx.lock().await.clone();
    if let Some(tx) = tx {
        tx.send(peer_id).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Coordinator not running".into())
    }
}

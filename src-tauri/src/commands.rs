use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use serde::Deserialize;
use tauri::State;

use crate::coordinator::Coordinator;
use crate::engine::screen_layout::{NeighborSide, ScreenDimensions};

// Store the JoinHandle so we can abort the spawned event loop.
// The Coordinator's Drop impl will ensure cleanup (stop capture, stop network).
pub struct AppState(pub Arc<Mutex<Option<JoinHandle<()>>>>);

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
) -> Result<(), String> {
    let mut lock = state.0.lock().await;

    if lock.is_some() {
        return Err("A coordinator is already running".into());
    }

    let mut coordinator = Coordinator::new_server(ScreenDimensions { width, height }, side.into());
    
    let handle = tokio::spawn(async move {
        // Run until aborted
        let _ = coordinator.run_as_server(&name).await;
    });

    *lock = Some(handle);
    Ok(())
}

#[tauri::command]
pub async fn start_client(
    width: u32,
    height: u32,
    side: SideParam,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut lock = state.0.lock().await;

    if lock.is_some() {
        return Err("A coordinator is already running".into());
    }

    let mut coordinator = Coordinator::new_client(ScreenDimensions { width, height }, side.into());

    let handle = tokio::spawn(async move {
        let _ = coordinator.run_as_client().await;
    });

    *lock = Some(handle);
    Ok(())
}

#[tauri::command]
pub async fn stop_coordinator(state: State<'_, AppState>) -> Result<(), String> {
    let mut lock = state.0.lock().await;
    if let Some(handle) = lock.take() {
        handle.abort();
    }
    Ok(())
}

use super::core::screen_layout::Point;

#[derive(Debug, Clone)]
pub enum InputEvent {
    MouseMove(Point),
    MouseButton { button: u8, pressed: bool },
    Scroll { dx: f32, dy: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionStatus {
    Granted,
    Denied,
    NotDetermined,
}

pub trait InputCapture: Send {
    fn start(&mut self, tx: tokio::sync::mpsc::Sender<InputEvent>) -> Result<(), InputError>;
    fn stop(&mut self);
    fn permission_status() -> PermissionStatus;
    fn request_permission();
}

pub trait InputInjector: Send {
    fn inject_move(&self, x_norm: f32, y_norm: f32);
    fn inject_button(&self, button: u8, pressed: bool);
    fn inject_scroll(&self, dx: f32, dy: f32);
    fn hide_cursor(&self);
    fn show_cursor(&self);
}

#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("accessibility permission denied")]
    PermissionDenied,
    #[error("failed to create event tap: {0}")]
    EventTapFailed(String),
    #[error("platform error: {0}")]
    Platform(String),
}

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

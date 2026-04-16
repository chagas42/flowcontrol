use super::engine::screen_layout::Point;

#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Absolute cursor position — used in Local/Transitioning state.
    MouseMove(Point),
    /// Raw hardware delta — sent when suppressing=true (Remote state).
    /// CGEventGetLocation is frozen in suppressing mode; deltas are always valid.
    /// `button`: None = pure move, Some(0) = left drag, Some(1) = right drag.
    MouseDelta { dx: f64, dy: f64, button: Option<u8> },
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
    fn permission_status(&self) -> PermissionStatus;
    fn request_permission(&self);
}

pub trait InputInjector: Send {
    /// Inject a cursor move. `button` carries drag state: None = move,
    /// Some(0) = left drag, Some(1) = right drag.
    fn inject_move(&self, pos: Point, button: Option<u8>);
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

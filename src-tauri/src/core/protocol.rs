use serde::{Deserialize, Serialize};

/// Wire format for all messages exchanged between machines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    MouseMove { x_norm: f32, y_norm: f32 },
    MouseButton { button: u8, pressed: bool },
    Scroll { dx: f32, dy: f32 },
    TransitionIn { y_norm: f32 },
    TransitionOut,
    ScreenInfo { width: u32, height: u32 },
}

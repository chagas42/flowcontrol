use serde::{Deserialize, Serialize};

use super::screen_layout::ScreenDimensions;

/// Wire format for all messages exchanged between machines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// `button`: None = pure move, Some(0) = left drag, Some(1) = right drag.
    MouseMove { x_norm: f32, y_norm: f32, button: Option<u8> },
    MouseButton { button: u8, pressed: bool },
    Scroll { dx: f32, dy: f32 },
    TransitionIn { y_norm: f32 },
    TransitionOut,
    /// Acknowledges receipt of TransitionIn. Sender moves to Remote + StartForwarding.
    Ack,
    ScreenInfo(ScreenDimensions),
}

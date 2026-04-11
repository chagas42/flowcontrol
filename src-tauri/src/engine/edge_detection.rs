use super::screen_layout::{Edge, Point, ScreenDimensions};

#[derive(Debug, Clone)]
pub struct EdgeCrossedEvent {
    pub edge: Edge,
    pub y_norm: f32,
    pub x_norm: f32,
}

pub trait EdgeDetection: Send + Sync {
    fn configure(&mut self, edge: Edge, bounds: ScreenDimensions, threshold: f64);
    fn update(&mut self, pos: Point) -> Option<EdgeCrossedEvent>;
}

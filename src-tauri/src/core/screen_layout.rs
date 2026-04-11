/// Physical side where the neighbor machine is located.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighborSide {
    Left,
    Right,
    Top,
    Bottom,
}

/// The edge of the local screen that triggers a transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy)]
pub struct ScreenDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct NormalizedPoint {
    pub x: f32,
    pub y: f32,
}

pub trait ScreenLayout: Send + Sync {
    fn configure(
        &mut self,
        side: NeighborSide,
        local: ScreenDimensions,
        remote: ScreenDimensions,
    );
    fn map_to_remote(&self, local: Point) -> NormalizedPoint;
    fn map_to_local(&self, norm: NormalizedPoint) -> Point;
    fn watched_edge(&self) -> Option<Edge>;
}

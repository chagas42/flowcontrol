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

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
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

pub struct ScreenLayoutImpl {
    side: Option<NeighborSide>,
    local: ScreenDimensions,
    remote: ScreenDimensions,
}

impl ScreenLayoutImpl {
    pub fn new() -> Self {
        Self {
            side: None,
            local: ScreenDimensions { width: 0, height: 0 },
            remote: ScreenDimensions { width: 0, height: 0 },
        }
    }
}

impl ScreenLayout for ScreenLayoutImpl {
    fn configure(&mut self, side: NeighborSide, local: ScreenDimensions, remote: ScreenDimensions) {
        self.side = Some(side);
        self.local = local;
        self.remote = remote;
    }

    fn map_to_remote(&self, local: Point) -> NormalizedPoint {
        NormalizedPoint {
            x: (local.x / self.local.width as f64) as f32,
            y: (local.y / self.local.height as f64) as f32,
        }
    }

    fn map_to_local(&self, norm: NormalizedPoint) -> Point {
        Point {
            x: norm.x as f64 * self.local.width as f64,
            y: norm.y as f64 * self.local.height as f64,
        }
    }

    fn watched_edge(&self) -> Option<Edge> {
        self.side.map(|s| match s {
            NeighborSide::Left => Edge::Left,
            NeighborSide::Right => Edge::Right,
            NeighborSide::Top => Edge::Top,
            NeighborSide::Bottom => Edge::Bottom,
        })
    }
}

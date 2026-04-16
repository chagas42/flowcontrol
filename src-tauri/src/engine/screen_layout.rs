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

/// Maps the side where the neighbor sits to the local screen edge that triggers a transition.
/// Right neighbor → watch the Right edge, etc.
pub fn neighbor_side_to_edge(side: NeighborSide) -> Edge {
    match side {
        NeighborSide::Left => Edge::Left,
        NeighborSide::Right => Edge::Right,
        NeighborSide::Top => Edge::Top,
        NeighborSide::Bottom => Edge::Bottom,
    }
}

/// Returns the edge opposite to the one watched — used to detect when the cursor
/// returns to the local screen while in Remote state.
pub fn opposite_edge(side: NeighborSide) -> Edge {
    match side {
        NeighborSide::Left => Edge::Right,
        NeighborSide::Right => Edge::Left,
        NeighborSide::Top => Edge::Bottom,
        NeighborSide::Bottom => Edge::Top,
    }
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
        self.side.map(neighbor_side_to_edge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neighbor_side_to_edge_all_variants() {
        assert_eq!(neighbor_side_to_edge(NeighborSide::Left), Edge::Left);
        assert_eq!(neighbor_side_to_edge(NeighborSide::Right), Edge::Right);
        assert_eq!(neighbor_side_to_edge(NeighborSide::Top), Edge::Top);
        assert_eq!(neighbor_side_to_edge(NeighborSide::Bottom), Edge::Bottom);
    }

    #[test]
    fn test_opposite_edge_all_variants() {
        assert_eq!(opposite_edge(NeighborSide::Left), Edge::Right);
        assert_eq!(opposite_edge(NeighborSide::Right), Edge::Left);
        assert_eq!(opposite_edge(NeighborSide::Top), Edge::Bottom);
        assert_eq!(opposite_edge(NeighborSide::Bottom), Edge::Top);
    }

    #[test]
    fn test_map_roundtrip() {
        let mut layout = ScreenLayoutImpl::new();
        layout.configure(
            NeighborSide::Right,
            ScreenDimensions { width: 1920, height: 1080 },
            ScreenDimensions { width: 2560, height: 1440 },
        );
        let norm = NormalizedPoint { x: 0.25, y: 0.75 };
        let local = layout.map_to_local(norm);
        let back = layout.map_to_remote(local);
        assert!((back.x - norm.x).abs() < 1e-5);
        assert!((back.y - norm.y).abs() < 1e-5);
    }
}

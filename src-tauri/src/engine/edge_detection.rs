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

pub struct EdgeDetectionImpl {
    edge: Edge,
    bounds: ScreenDimensions,
    threshold: f64,
    was_at_edge: bool,
    configured: bool,
}

impl EdgeDetectionImpl {
    pub fn new() -> Self {
        Self {
            edge: Edge::Right,
            bounds: ScreenDimensions { width: 0, height: 0 },
            threshold: 0.0,
            was_at_edge: false,
            configured: false,
        }
    }

    fn is_at_edge(&self, pos: Point) -> bool {
        match self.edge {
            Edge::Right => pos.x >= self.bounds.width as f64 - self.threshold,
            Edge::Left => pos.x <= self.threshold,
            Edge::Bottom => pos.y >= self.bounds.height as f64 - self.threshold,
            Edge::Top => pos.y <= self.threshold,
        }
    }
}

impl EdgeDetection for EdgeDetectionImpl {
    fn configure(&mut self, edge: Edge, bounds: ScreenDimensions, threshold: f64) {
        self.edge = edge;
        self.bounds = bounds;
        self.threshold = threshold;
        self.was_at_edge = false;
        self.configured = true;
    }

    fn update(&mut self, pos: Point) -> Option<EdgeCrossedEvent> {
        if !self.configured {
            return None;
        }
        let now_at_edge = self.is_at_edge(pos);
        if now_at_edge && !self.was_at_edge {
            self.was_at_edge = true;
            Some(EdgeCrossedEvent {
                edge: self.edge,
                x_norm: (pos.x / self.bounds.width as f64) as f32,
                y_norm: (pos.y / self.bounds.height as f64) as f32,
            })
        } else {
            if !now_at_edge {
                self.was_at_edge = false;
            }
            None
        }
    }
}

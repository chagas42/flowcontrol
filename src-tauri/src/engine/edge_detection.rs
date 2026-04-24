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
            bounds: ScreenDimensions {
                width: 0,
                height: 0,
            },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::edge_detection::EdgeDetection;

    fn dims(w: u32, h: u32) -> ScreenDimensions {
        ScreenDimensions {
            width: w,
            height: h,
        }
    }

    #[test]
    fn test_unconfigured_returns_none() {
        let mut ed = EdgeDetectionImpl::new();
        assert!(ed.update(Point { x: 1919.0, y: 0.0 }).is_none());
    }

    #[test]
    fn test_fires_on_crossing_right_edge() {
        let mut ed = EdgeDetectionImpl::new();
        ed.configure(Edge::Right, dims(1920, 1080), 10.0);
        assert!(ed
            .update(Point {
                x: 1000.0,
                y: 540.0
            })
            .is_none());
        let ev = ed
            .update(Point {
                x: 1915.0,
                y: 540.0,
            })
            .expect("should fire");
        assert_eq!(ev.edge, Edge::Right);
        assert!((ev.y_norm - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_resets_after_leaving_edge() {
        let mut ed = EdgeDetectionImpl::new();
        ed.configure(Edge::Right, dims(1920, 1080), 10.0);
        ed.update(Point { x: 1915.0, y: 0.0 }); // fire
        ed.update(Point { x: 100.0, y: 0.0 }); // leave → reset
        assert!(
            ed.update(Point { x: 1919.0, y: 0.0 }).is_some(),
            "should fire again after reset"
        );
    }

    #[test]
    fn test_no_double_fire() {
        let mut ed = EdgeDetectionImpl::new();
        ed.configure(Edge::Right, dims(1920, 1080), 10.0);
        ed.update(Point { x: 1915.0, y: 0.0 }); // first fire
        assert!(
            ed.update(Point { x: 1919.0, y: 0.0 }).is_none(),
            "should not fire twice"
        );
    }
}

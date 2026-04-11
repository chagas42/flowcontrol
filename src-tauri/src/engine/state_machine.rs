use super::edge_detection::EdgeCrossedEvent;
use super::protocol::Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Local,
    Transitioning,
    Remote,
}

#[derive(Debug)]
pub enum Event {
    EdgeCrossed(EdgeCrossedEvent),
    TransitionAcknowledged,
    TransitionInReceived { y_norm: f32 },
    ConnectionEstablished,
    ConnectionLost,
}

#[derive(Debug)]
pub enum Command {
    StartForwarding,
    StopForwarding,
    AcceptCursor { y_norm: f32 },
    Send(Message),
}

pub trait StateMachine: Send {
    fn handle(&mut self, event: Event) -> Vec<Command>;
    fn state(&self) -> State;
    fn reset(&mut self);
}

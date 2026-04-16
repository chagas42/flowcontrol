use super::edge_detection::EdgeCrossedEvent;
use super::protocol::Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Local,
    Transitioning,
    Remote,
    /// Server has detected cursor returned to local screen and sent TransitionIn to client.
    /// Waiting for client's Ack before re-enabling edge detection.
    ReturnTransitioning,
}

#[derive(Debug)]
pub enum Event {
    EdgeCrossed(EdgeCrossedEvent),
    TransitionAcknowledged,
    TransitionInReceived { y_norm: f32 },
    /// Server detected virtual_pos crossed back to local screen while in Remote state.
    CursorReturnedToLocal { y_norm: f32 },
    ConnectionEstablished,
    ConnectionLost,
}

#[derive(Debug)]
pub enum Command {
    StartForwarding,
    StopForwarding,
    AcceptCursor { y_norm: f32 },
    /// Stop suppression and warp cursor to the return entry point on the local screen.
    ReturnCursorToLocal { y_norm: f32 },
    Send(Message),
}

pub trait StateMachine: Send {
    fn handle(&mut self, event: Event) -> Vec<Command>;
    fn state(&self) -> State;
    fn reset(&mut self);
}

pub struct StateMachineImpl {
    state: State,
}

impl StateMachineImpl {
    pub fn new() -> Self {
        Self { state: State::Local }
    }

    pub fn new_as_client() -> Self {
        Self { state: State::Remote }
    }
}

impl StateMachine for StateMachineImpl {
    fn handle(&mut self, event: Event) -> Vec<Command> {
        match (&self.state, event) {
            (State::Local, Event::EdgeCrossed(e)) => {
                self.state = State::Transitioning;
                vec![Command::Send(Message::TransitionIn { y_norm: e.y_norm })]
            }
            (State::Transitioning, Event::TransitionAcknowledged) => {
                self.state = State::Remote;
                vec![Command::StartForwarding]
            }
            (State::Remote, Event::TransitionInReceived { y_norm }) => {
                self.state = State::Local;
                vec![
                    Command::StopForwarding,
                    Command::AcceptCursor { y_norm },
                    Command::Send(Message::Ack),
                ]
            }
            (State::Local, Event::TransitionInReceived { .. }) => {
                // Client had the cursor (Local) and server is recalling it.
                self.state = State::Remote;
                vec![Command::StartForwarding, Command::Send(Message::Ack)]
            }
            (State::Remote, Event::CursorReturnedToLocal { y_norm }) => {
                self.state = State::ReturnTransitioning;
                vec![
                    Command::Send(Message::TransitionIn { y_norm }),
                    Command::ReturnCursorToLocal { y_norm },
                ]
            }
            (State::ReturnTransitioning, Event::TransitionAcknowledged) => {
                self.state = State::Local;
                vec![]
            }
            (State::Remote, Event::ConnectionLost) => {
                self.state = State::Local;
                vec![Command::StopForwarding]
            }
            (State::ReturnTransitioning, Event::ConnectionLost) => {
                self.state = State::Local;
                vec![]
            }
            (_, Event::ConnectionLost) => {
                self.state = State::Local;
                vec![]
            }
            _ => vec![],
        }
    }

    fn state(&self) -> State {
        self.state
    }

    fn reset(&mut self) {
        self.state = State::Local;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{
        edge_detection::EdgeCrossedEvent,
        screen_layout::Edge,
    };

    fn make_crossed(y: f32) -> EdgeCrossedEvent {
        EdgeCrossedEvent { edge: Edge::Right, x_norm: 1.0, y_norm: y }
    }

    #[test]
    fn test_forward_sequence() {
        let mut sm = StateMachineImpl::new();
        assert_eq!(sm.state(), State::Local);

        let cmds = sm.handle(Event::EdgeCrossed(make_crossed(0.5)));
        assert_eq!(sm.state(), State::Transitioning);
        assert!(matches!(cmds[..], [Command::Send(_)]));

        let cmds = sm.handle(Event::TransitionAcknowledged);
        assert_eq!(sm.state(), State::Remote);
        assert!(matches!(cmds[..], [Command::StartForwarding]));
    }

    #[test]
    fn test_return_sequence() {
        let mut sm = StateMachineImpl::new();
        sm.state = State::Remote;

        let cmds = sm.handle(Event::CursorReturnedToLocal { y_norm: 0.4 });
        assert_eq!(sm.state(), State::ReturnTransitioning);
        assert_eq!(cmds.len(), 2);
        assert!(matches!(cmds[0], Command::Send(_)));
        assert!(matches!(cmds[1], Command::ReturnCursorToLocal { y_norm } if (y_norm - 0.4).abs() < f32::EPSILON));

        let cmds = sm.handle(Event::TransitionAcknowledged);
        assert_eq!(sm.state(), State::Local);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_client_transition_in() {
        let mut sm = StateMachineImpl::new_as_client();
        assert_eq!(sm.state(), State::Remote);

        let cmds = sm.handle(Event::TransitionInReceived { y_norm: 0.6 });
        assert_eq!(sm.state(), State::Local);
        assert_eq!(cmds.len(), 3);
        assert!(matches!(cmds[0], Command::StopForwarding));
        assert!(matches!(cmds[1], Command::AcceptCursor { .. }));
        assert!(matches!(cmds[2], Command::Send(_)));
    }

    #[test]
    fn test_connection_lost_from_all_states() {
        for start in [State::Local, State::Transitioning, State::Remote, State::ReturnTransitioning] {
            let mut sm = StateMachineImpl { state: start };
            sm.handle(Event::ConnectionLost);
            assert_eq!(sm.state(), State::Local, "ConnectionLost from {start:?} should → Local");
        }
    }

    #[test]
    fn test_client_return_transition() {
        // Client had cursor (Local), server recalls it via TransitionIn.
        let mut sm = StateMachineImpl::new();
        sm.state = State::Local;
        let cmds = sm.handle(Event::TransitionInReceived { y_norm: 0.5 });
        assert_eq!(sm.state(), State::Remote);
        assert_eq!(cmds.len(), 2);
        assert!(matches!(cmds[0], Command::StartForwarding));
        assert!(matches!(cmds[1], Command::Send(_)));
    }

    #[test]
    fn test_full_round_trip_twice() {
        // Simulates two server + client state machines going through 2 full cycles.
        let mut server = StateMachineImpl::new();          // starts Local
        let mut client = StateMachineImpl::new_as_client(); // starts Remote

        for _ in 0..2 {
            // Forward: server detects edge
            let cmds = server.handle(Event::EdgeCrossed(make_crossed(0.5)));
            assert_eq!(server.state(), State::Transitioning);
            assert!(matches!(cmds[0], Command::Send(Message::TransitionIn { .. })));

            // Client receives TransitionIn (Remote → Local)
            let cmds = client.handle(Event::TransitionInReceived { y_norm: 0.5 });
            assert_eq!(client.state(), State::Local);
            assert!(matches!(cmds[2], Command::Send(Message::Ack)));

            // Server receives Ack (Transitioning → Remote)
            let cmds = server.handle(Event::TransitionAcknowledged);
            assert_eq!(server.state(), State::Remote);
            assert!(matches!(cmds[0], Command::StartForwarding));

            // Return: server detects cursor back
            let cmds = server.handle(Event::CursorReturnedToLocal { y_norm: 0.5 });
            assert_eq!(server.state(), State::ReturnTransitioning);
            assert!(matches!(cmds[0], Command::Send(Message::TransitionIn { .. })));

            // Client receives TransitionIn (Local → Remote)
            let cmds = client.handle(Event::TransitionInReceived { y_norm: 0.5 });
            assert_eq!(client.state(), State::Remote);
            assert!(matches!(cmds[1], Command::Send(Message::Ack)));

            // Server receives Ack (ReturnTransitioning → Local)
            server.handle(Event::TransitionAcknowledged);
            assert_eq!(server.state(), State::Local);
        }
    }

    #[test]
    fn test_stray_events_ignored() {
        let mut sm = StateMachineImpl::new();
        sm.state = State::Remote;
        let cmds = sm.handle(Event::EdgeCrossed(make_crossed(0.5)));
        assert!(cmds.is_empty());
        assert_eq!(sm.state(), State::Remote);
    }
}

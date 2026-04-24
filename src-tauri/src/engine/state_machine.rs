use super::edge_detection::EdgeCrossedEvent;
use super::protocol::Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// New: TCP is up but user has not approved the pair yet. Coordinate I/O
    /// is gated off while in this state; the coordinator just emits the
    /// pair-incoming event and waits for Accept/Decline or a 30s timeout.
    Pairing,
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
    TransitionInReceived {
        y_norm: f32,
    },
    /// Server detected virtual_pos crossed back to local screen while in Remote state.
    CursorReturnedToLocal {
        y_norm: f32,
    },
    ConnectionEstablished,
    ConnectionLost,
    /// User tapped Accept (local or remote side delivered a PairAccept message).
    PairAccepted,
    /// User tapped Decline, remote declined, or the 30s timer fired.
    /// Behaves like ConnectionLost for the state machine.
    PairDeclined,
}

#[derive(Debug)]
pub enum Command {
    StartForwarding,
    StopForwarding,
    AcceptCursor {
        y_norm: f32,
    },
    /// Stop suppression and warp cursor to the return entry point on the local screen.
    ReturnCursorToLocal {
        y_norm: f32,
    },
    Send(Message),
}

pub trait StateMachine: Send {
    fn handle(&mut self, event: Event) -> Vec<Command>;
    fn state(&self) -> State;
    fn reset(&mut self);
}

pub struct StateMachineImpl {
    state: State,
    /// The state we return to after a successful PairAccepted — `Local` for
    /// the server (mouse-owning side), `Remote` for the client.
    home: State,
}

impl StateMachineImpl {
    pub fn new() -> Self {
        Self {
            state: State::Local,
            home: State::Local,
        }
    }

    pub fn new_as_client() -> Self {
        Self {
            state: State::Remote,
            home: State::Remote,
        }
    }
}

impl StateMachine for StateMachineImpl {
    fn handle(&mut self, event: Event) -> Vec<Command> {
        match (&self.state, event) {
            // TCP came up. Gate I/O behind user approval on both sides.
            (_, Event::ConnectionEstablished) => {
                self.state = State::Pairing;
                vec![]
            }
            // User (or remote) approved. Land back in our home state.
            (State::Pairing, Event::PairAccepted) => {
                self.state = self.home;
                vec![]
            }
            // Either side declined, or 30s timeout fired.
            (State::Pairing, Event::PairDeclined) => {
                self.state = self.home;
                vec![]
            }
            (State::Local, Event::EdgeCrossed(e)) => {
                self.state = State::Remote;
                vec![
                    Command::StartForwarding,
                    Command::Send(Message::TransitionIn { y_norm: e.y_norm }),
                ]
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
                // No side effects needed — cursor disappears when the server
                // warps it back via ReturnCursorToLocal on its side.
                self.state = State::Remote;
                vec![Command::Send(Message::Ack)]
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
            (State::Remote, Event::ConnectionLost) if self.home == State::Local => {
                // Server lost the peer while forwarding — stop suppression and return to Local.
                self.state = State::Local;
                vec![Command::StopForwarding]
            }
            (_, Event::ConnectionLost) => {
                self.state = self.home;
                vec![]
            }
            _ => vec![],
        }
    }

    fn state(&self) -> State {
        self.state
    }

    fn reset(&mut self) {
        self.state = self.home;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{edge_detection::EdgeCrossedEvent, screen_layout::Edge};

    fn make_crossed(y: f32) -> EdgeCrossedEvent {
        EdgeCrossedEvent {
            edge: Edge::Right,
            x_norm: 1.0,
            y_norm: y,
        }
    }

    #[test]
    fn test_forward_sequence() {
        let mut sm = StateMachineImpl::new();
        assert_eq!(sm.state(), State::Local);

        let cmds = sm.handle(Event::EdgeCrossed(make_crossed(0.5)));
        assert_eq!(sm.state(), State::Remote);
        assert_eq!(cmds.len(), 2);
        assert!(matches!(cmds[0], Command::StartForwarding));
        assert!(matches!(cmds[1], Command::Send(_)));

        // TransitionAcknowledged is now a no-op in Remote
        let cmds = sm.handle(Event::TransitionAcknowledged);
        assert_eq!(sm.state(), State::Remote);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_return_sequence() {
        let mut sm = StateMachineImpl::new();
        sm.state = State::Remote;

        let cmds = sm.handle(Event::CursorReturnedToLocal { y_norm: 0.4 });
        assert_eq!(sm.state(), State::ReturnTransitioning);
        assert_eq!(cmds.len(), 2);
        assert!(matches!(cmds[0], Command::Send(_)));
        assert!(
            matches!(cmds[1], Command::ReturnCursorToLocal { y_norm } if (y_norm - 0.4).abs() < f32::EPSILON)
        );

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
        for start in [
            State::Pairing,
            State::Local,
            State::Transitioning,
            State::Remote,
            State::ReturnTransitioning,
        ] {
            let mut sm = StateMachineImpl {
                state: start,
                home: State::Local,
            };
            sm.handle(Event::ConnectionLost);
            assert_eq!(
                sm.state(),
                State::Local,
                "ConnectionLost from {start:?} should → home (Local)"
            );
        }
    }

    #[test]
    fn test_connection_established_enters_pairing() {
        let mut server = StateMachineImpl::new();
        server.handle(Event::ConnectionEstablished);
        assert_eq!(server.state(), State::Pairing);

        let mut client = StateMachineImpl::new_as_client();
        client.handle(Event::ConnectionEstablished);
        assert_eq!(client.state(), State::Pairing);
    }

    #[test]
    fn test_pair_accept_returns_to_home() {
        let mut server = StateMachineImpl::new();
        server.handle(Event::ConnectionEstablished);
        server.handle(Event::PairAccepted);
        assert_eq!(server.state(), State::Local);

        let mut client = StateMachineImpl::new_as_client();
        client.handle(Event::ConnectionEstablished);
        client.handle(Event::PairAccepted);
        assert_eq!(client.state(), State::Remote);
    }

    #[test]
    fn test_pair_decline_returns_to_home() {
        let mut client = StateMachineImpl::new_as_client();
        client.handle(Event::ConnectionEstablished);
        client.handle(Event::PairDeclined);
        assert_eq!(client.state(), State::Remote);
    }

    #[test]
    fn test_client_return_transition() {
        // Client had cursor (Local), server recalls it via TransitionIn.
        let mut sm = StateMachineImpl::new();
        sm.state = State::Local;
        let cmds = sm.handle(Event::TransitionInReceived { y_norm: 0.5 });
        assert_eq!(sm.state(), State::Remote);
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0], Command::Send(Message::Ack)));
    }

    #[test]
    fn test_full_round_trip_twice() {
        // Simulates two server + client state machines going through 2 full cycles.
        let mut server = StateMachineImpl::new(); // starts Local
        let mut client = StateMachineImpl::new_as_client(); // starts Remote

        for _ in 0..2 {
            // Forward: server detects edge
            let cmds = server.handle(Event::EdgeCrossed(make_crossed(0.5)));
            assert_eq!(server.state(), State::Remote);
            assert_eq!(cmds.len(), 2);
            assert!(matches!(cmds[0], Command::StartForwarding));
            assert!(matches!(
                cmds[1],
                Command::Send(Message::TransitionIn { .. })
            ));

            // Client receives TransitionIn (Remote → Local)
            let cmds = client.handle(Event::TransitionInReceived { y_norm: 0.5 });
            assert_eq!(client.state(), State::Local);
            assert!(matches!(cmds[2], Command::Send(Message::Ack)));

            // Server receives Ack — now a no-op (already in Remote)
            let cmds = server.handle(Event::TransitionAcknowledged);
            assert_eq!(server.state(), State::Remote);
            assert!(cmds.is_empty());

            // Return: server detects cursor back
            let cmds = server.handle(Event::CursorReturnedToLocal { y_norm: 0.5 });
            assert_eq!(server.state(), State::ReturnTransitioning);
            assert!(matches!(
                cmds[0],
                Command::Send(Message::TransitionIn { .. })
            ));

            // Client receives TransitionIn (Local → Remote)
            let cmds = client.handle(Event::TransitionInReceived { y_norm: 0.5 });
            assert_eq!(client.state(), State::Remote);
            assert!(matches!(cmds[0], Command::Send(Message::Ack)));

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

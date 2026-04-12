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
            (State::Remote, Event::ConnectionLost) => {
                self.state = State::Local;
                vec![Command::StopForwarding]
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

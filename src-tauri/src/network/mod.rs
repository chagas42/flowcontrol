use crate::engine::protocol::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Browsing,
    Advertising,
    Connecting,
    Connected,
}

#[derive(Debug, Clone)]
pub struct Peer {
    pub id: String,
    pub name: String,
}

pub trait NetworkLayer: Send {
    async fn start_server(&mut self, name: &str) -> Result<(), NetworkError>;
    async fn start_client(&mut self) -> Result<(), NetworkError>;
    async fn connect(&mut self, peer: &Peer) -> Result<(), NetworkError>;
    async fn send(&self, msg: Message) -> Result<(), NetworkError>;
    fn state(&self) -> ConnectionState;
    fn peers(&self) -> Vec<Peer>;
    fn stop(&mut self);
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("send failed: {0}")]
    SendFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

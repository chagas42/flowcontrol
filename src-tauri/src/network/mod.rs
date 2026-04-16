use crate::engine::protocol::Message;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
};

const PORT: u16 = 7878;
const SERVICE_TYPE: &str = "_flowcontrol._tcp.local.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Browsing,
    Advertising,
    Connecting,
    Connected,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

/// Events emitted by the network layer to the coordinator.
#[derive(Debug)]
pub enum NetworkEvent {
    MessageReceived(Message),
    StateChanged(ConnectionState),
    PeersUpdated(Vec<Peer>),
}

pub struct NetworkLayerImpl {
    state: Arc<Mutex<ConnectionState>>,
    peers: Arc<Mutex<Vec<Peer>>>,
    /// Maps peer id (mDNS fullname) to resolved socket address. Internal only.
    peer_addrs: Arc<Mutex<HashMap<String, SocketAddr>>>,
    event_tx: mpsc::Sender<NetworkEvent>,
    /// Write channel to the active TCP connection's writer task.
    write_tx: Arc<Mutex<Option<mpsc::Sender<Message>>>>,
    shutdown_tx: Option<broadcast::Sender<()>>,
    mdns: Option<ServiceDaemon>,
    /// Full mDNS service name for unregistering on stop.
    mdns_fullname: Option<String>,
}

impl NetworkLayerImpl {
    pub fn new(event_tx: mpsc::Sender<NetworkEvent>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            peers: Arc::new(Mutex::new(Vec::new())),
            peer_addrs: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            write_tx: Arc::new(Mutex::new(None)),
            shutdown_tx: None,
            mdns: None,
            mdns_fullname: None,
        }
    }

    fn set_state(&self, new_state: ConnectionState) {
        *self.state.lock().unwrap() = new_state.clone();
        let _ = self.event_tx.try_send(NetworkEvent::StateChanged(new_state));
    }

    /// Returns the local IP used for routing to the internet (does not send any packets).
    fn local_ip() -> Option<IpAddr> {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.connect("8.8.8.8:80").ok()?;
        Some(socket.local_addr().ok()?.ip())
    }

    /// Splits a connected TCP stream into a reader task and writer task.
    /// Stores the message sender in `write_tx_slot`.
    fn spawn_connection(
        stream: TcpStream,
        event_tx: mpsc::Sender<NetworkEvent>,
        state: Arc<Mutex<ConnectionState>>,
        write_tx_slot: Arc<Mutex<Option<mpsc::Sender<Message>>>>,
        shutdown_tx: broadcast::Sender<()>,
    ) {
        let (msg_tx, mut msg_rx) = mpsc::channel::<Message>(64);
        *write_tx_slot.lock().unwrap() = Some(msg_tx);

        let (mut read_half, mut write_half) = stream.into_split();
        let mut rd_shutdown = shutdown_tx.subscribe();
        let mut wr_shutdown = shutdown_tx.subscribe();

        // Reader task: length-prefixed bincode frames → NetworkEvent::MessageReceived
        let event_tx_r = event_tx.clone();
        let state_r = state.clone();
        tokio::spawn(async move {
            'read: loop {
                let mut len_buf = [0u8; 4];
                tokio::select! {
                    biased;
                    _ = rd_shutdown.recv() => break 'read,
                    result = read_half.read_exact(&mut len_buf) => {
                        if result.is_err() { break 'read; }
                        let len = u32::from_be_bytes(len_buf) as usize;
                        let mut buf = vec![0u8; len];
                        if read_half.read_exact(&mut buf).await.is_err() { break 'read; }
                        match bincode::deserialize::<Message>(&buf) {
                            Ok(msg) => {
                                let _ = event_tx_r.send(NetworkEvent::MessageReceived(msg)).await;
                            }
                            Err(_) => break 'read,
                        }
                    }
                }
            }
            *state_r.lock().unwrap() = ConnectionState::Disconnected;
            let _ = event_tx_r
                .send(NetworkEvent::StateChanged(ConnectionState::Disconnected))
                .await;
        });

        // Writer task: Message → length-prefixed bincode frame → socket
        tokio::spawn(async move {
            let mut buf = Vec::with_capacity(512);
            loop {
                tokio::select! {
                    biased;
                    _ = wr_shutdown.recv() => break,
                    msg = msg_rx.recv() => {
                        let Some(msg) = msg else { break };
                        buf.clear();
                        buf.extend_from_slice(&[0u8; 4]);
                        if bincode::serialize_into(&mut buf, &msg).is_err() { continue; }
                        let len = ((buf.len() - 4) as u32).to_be_bytes();
                        buf[..4].copy_from_slice(&len);
                        if write_half.write_all(&buf).await.is_err() { break; }
                    }
                }
            }
        });
    }
}

impl NetworkLayer for NetworkLayerImpl {
    async fn start_server(&mut self, name: &str) -> Result<(), NetworkError> {
        let (shutdown_tx, _) = broadcast::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx.clone());

        let listener = TcpListener::bind(format!("0.0.0.0:{PORT}")).await?;

        let daemon =
            ServiceDaemon::new().map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        let ip = Self::local_ip()
            .ok_or_else(|| NetworkError::ConnectionFailed("cannot determine local IP".into()))?;

        // DNS hostnames must not contain spaces or special chars.
        let host_slug: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();
        let host_name = format!("{host_slug}.local.");
        let service_info = ServiceInfo::new(SERVICE_TYPE, name, &host_name, ip, PORT, None)
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        let fullname = service_info.get_fullname().to_string();
        daemon
            .register(service_info)
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        self.mdns = Some(daemon);
        self.mdns_fullname = Some(fullname);
        self.set_state(ConnectionState::Advertising);

        let event_tx = self.event_tx.clone();
        let state = self.state.clone();
        let write_tx_slot = self.write_tx.clone();
        let mut accept_shutdown = shutdown_tx.subscribe();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    _ = accept_shutdown.recv() => break,
                    result = listener.accept() => {
                        let Ok((stream, _)) = result else { break };
                        stream.set_nodelay(true).ok();
                        *state.lock().unwrap() = ConnectionState::Connected;
                        let _ = event_tx
                            .send(NetworkEvent::StateChanged(ConnectionState::Connected))
                            .await;
                        NetworkLayerImpl::spawn_connection(
                            stream,
                            event_tx.clone(),
                            state.clone(),
                            write_tx_slot.clone(),
                            shutdown_tx.clone(),
                        );
                        // v1: one connection at a time
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    async fn start_client(&mut self) -> Result<(), NetworkError> {
        let (shutdown_tx, _) = broadcast::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx.clone());

        let daemon =
            ServiceDaemon::new().map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        let receiver = daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        self.mdns = Some(daemon);
        self.set_state(ConnectionState::Browsing);

        let event_tx = self.event_tx.clone();
        let peers = self.peers.clone();
        let peer_addrs = self.peer_addrs.clone();

        // Drive the blocking mdns receiver into an async channel.
        let (mdns_tx, mut mdns_rx) = mpsc::channel::<ServiceEvent>(32);
        tokio::task::spawn_blocking(move || {
            while let Ok(event) = receiver.recv() {
                if mdns_tx.blocking_send(event).is_err() {
                    break;
                }
            }
        });

        let mut browse_shutdown = shutdown_tx.subscribe();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    _ = browse_shutdown.recv() => break,
                    event = mdns_rx.recv() => {
                        let Some(event) = event else { break };
                        match event {
                            ServiceEvent::ServiceResolved(info) => {
                                let Some(ip) = info
                                    .get_addresses()
                                    .iter()
                                    .find(|a| matches!(a, IpAddr::V4(_)))
                                    .copied()
                                else {
                                    continue;
                                };
                                let addr = SocketAddr::new(ip, info.get_port());
                                let peer = Peer {
                                    id: info.get_fullname().to_string(),
                                    name: info
                                        .get_hostname()
                                        .trim_end_matches('.')
                                        .to_string(),
                                };
                                peers.lock().unwrap().push(peer.clone());
                                peer_addrs.lock().unwrap().insert(peer.id.clone(), addr);
                                let all = peers.lock().unwrap().clone();
                                let _ = event_tx.send(NetworkEvent::PeersUpdated(all)).await;
                            }
                            ServiceEvent::ServiceRemoved(_, fullname) => {
                                peers.lock().unwrap().retain(|p| p.id != fullname);
                                peer_addrs.lock().unwrap().remove(&fullname);
                                let all = peers.lock().unwrap().clone();
                                let _ = event_tx.send(NetworkEvent::PeersUpdated(all)).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        Ok(())
    }

    async fn connect(&mut self, peer: &Peer) -> Result<(), NetworkError> {
        let addr = self
            .peer_addrs
            .lock()
            .unwrap()
            .get(&peer.id)
            .copied()
            .ok_or_else(|| {
                NetworkError::ConnectionFailed(format!("unknown peer: {}", peer.id))
            })?;
        self.connect_addr(addr).await
    }

    async fn send(&self, msg: Message) -> Result<(), NetworkError> {
        let tx = self.write_tx.lock().unwrap().clone();
        match tx {
            Some(tx) => tx
                .send(msg)
                .await
                .map_err(|e| NetworkError::SendFailed(e.to_string())),
            None => Err(NetworkError::SendFailed("not connected".into())),
        }
    }

    fn state(&self) -> ConnectionState {
        self.state.lock().unwrap().clone()
    }

    fn peers(&self) -> Vec<Peer> {
        self.peers.lock().unwrap().clone()
    }

    fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(fullname) = self.mdns_fullname.take() {
            if let Some(daemon) = &self.mdns {
                let _ = daemon.unregister(&fullname);
            }
        }
        self.mdns = None;
        *self.write_tx.lock().unwrap() = None;
        *self.state.lock().unwrap() = ConnectionState::Disconnected;
        let _ = self
            .event_tx
            .try_send(NetworkEvent::StateChanged(ConnectionState::Disconnected));
        self.peers.lock().unwrap().clear();
        self.peer_addrs.lock().unwrap().clear();
    }
}

impl NetworkLayerImpl {
    /// Connect directly to a known IP:port, bypassing mDNS discovery.
    pub async fn connect_direct(&mut self, addr: std::net::SocketAddr) -> Result<(), NetworkError> {
        self.connect_addr(addr).await
    }

    async fn connect_addr(&mut self, addr: std::net::SocketAddr) -> Result<(), NetworkError> {
        self.set_state(ConnectionState::Connecting);

        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;
        stream.set_nodelay(true).ok();

        let shutdown_tx = self
            .shutdown_tx
            .get_or_insert_with(|| broadcast::channel::<()>(1).0)
            .clone();

        self.set_state(ConnectionState::Connected);

        Self::spawn_connection(
            stream,
            self.event_tx.clone(),
            self.state.clone(),
            self.write_tx.clone(),
            shutdown_tx,
        );

        Ok(())
    }
}

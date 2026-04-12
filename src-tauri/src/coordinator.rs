use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter};

use tokio::sync::mpsc;

use crate::engine::{
    edge_detection::EdgeDetectionImpl,
    protocol::Message,
    screen_layout::{NeighborSide, NormalizedPoint, ScreenDimensions, ScreenLayoutImpl},
    state_machine::{Command, Event, State, StateMachineImpl},
};
use crate::input::{InputCapture, InputEvent, InputInjector};
use crate::network::{ConnectionState, NetworkEvent, NetworkLayer, NetworkLayerImpl};

#[cfg(target_os = "macos")]
use crate::{
    engine::screen_layout::Edge,
    input::macos::{MacOSCapture, MacOSInjector},
};

use crate::engine::edge_detection::EdgeDetection;
use crate::engine::screen_layout::ScreenLayout;
use crate::engine::state_machine::StateMachine;
use crate::network::NetworkError;

// ---------------------------------------------------------------------------
// Coordinator
// ---------------------------------------------------------------------------

pub struct Coordinator {
    state_machine: StateMachineImpl,
    screen_layout: ScreenLayoutImpl,
    edge_detection: EdgeDetectionImpl,
    network: NetworkLayerImpl,

    #[cfg(target_os = "macos")]
    capture: MacOSCapture,
    #[cfg(target_os = "macos")]
    injector: MacOSInjector,

    /// Receiver end of the input event channel. The sender is given to `capture.start()`.
    input_rx: mpsc::Receiver<InputEvent>,
    /// Sender end stored so `run_*` methods can pass it to capture without
    /// creating a second channel.
    input_tx: mpsc::Sender<InputEvent>,
    /// Receiver end of the network event channel. The sender is owned by `network`.
    network_rx: mpsc::Receiver<NetworkEvent>,

    /// Local screen dimensions — sent as `ScreenInfo` once connected.
    local_dims: ScreenDimensions,
    /// Physical side of the screen where the remote machine sits.
    neighbor_side: NeighborSide,

    /// Channel to receive manual connection triggers from frontend
    connect_rx: mpsc::Receiver<String>,
    app_handle: Option<AppHandle>,
}

impl Coordinator {
    fn new_inner(
        state_machine: StateMachineImpl,
        local_dims: ScreenDimensions,
        side: NeighborSide,
        app_handle: Option<AppHandle>,
        connect_rx: mpsc::Receiver<String>,
    ) -> Self {
        let (input_tx, input_rx) = mpsc::channel::<InputEvent>(64);
        let (net_tx, network_rx) = mpsc::channel::<NetworkEvent>(32);

        Self {
            state_machine,
            screen_layout: ScreenLayoutImpl::new(),
            edge_detection: EdgeDetectionImpl::new(),
            network: NetworkLayerImpl::new(net_tx),

            #[cfg(target_os = "macos")]
            capture: MacOSCapture::new(),
            #[cfg(target_os = "macos")]
            injector: MacOSInjector::new(),

            input_rx,
            input_tx,
            network_rx,
            local_dims,
            neighbor_side: side,
            connect_rx,
            app_handle,
        }
    }

    /// Coordinator seeded as the server (physical mouse machine).
    /// State machine starts in `Local`.
    pub fn new_server(local_dims: ScreenDimensions, side: NeighborSide, app_handle: Option<AppHandle>, connect_rx: mpsc::Receiver<String>) -> Self {
        Self::new_inner(StateMachineImpl::new(), local_dims, side, app_handle, connect_rx)
    }

    /// Coordinator seeded as the client (display-only machine).
    /// State machine starts in `Remote`.
    pub fn new_client(local_dims: ScreenDimensions, side: NeighborSide, app_handle: Option<AppHandle>, connect_rx: mpsc::Receiver<String>) -> Self {
        Self::new_inner(StateMachineImpl::new_as_client(), local_dims, side, app_handle, connect_rx)
    }

    // -----------------------------------------------------------------------
    // Entry points
    // -----------------------------------------------------------------------

    /// Start as the server: advertise via mDNS, capture input, run event loop.
    pub async fn run_as_server(&mut self, name: &str) -> Result<(), NetworkError> {
        self.network.start_server(name).await?;

        #[cfg(target_os = "macos")]
        self.capture
            .start(self.input_tx.clone())
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        self.event_loop().await;
        Ok(())
    }

    /// Start as the client: browse mDNS, capture input, run event loop.
    /// Call `network.connect(peer)` separately after peers are discovered.
    pub async fn run_as_client(&mut self) -> Result<(), NetworkError> {
        self.network.start_client().await?;

        #[cfg(target_os = "macos")]
        {
            self.capture
                .start(self.input_tx.clone())
                .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;
            // Client starts in Remote — cursor is hidden until AcceptCursor fires.
            self.injector.hide_cursor();
        }

        self.event_loop().await;
        Ok(())
    }

    /// Graceful shutdown: stop capture, stop network.
    pub fn stop(&mut self) {
        #[cfg(target_os = "macos")]
        self.capture.stop();
        self.network.stop();
    }

    // -----------------------------------------------------------------------
    // Event loop
    // -----------------------------------------------------------------------

    async fn event_loop(&mut self) {
        loop {
            tokio::select! {
                ev = self.input_rx.recv() => {
                    let Some(ev) = ev else { break };
                    self.on_input(ev).await;
                }
                ev = self.network_rx.recv() => {
                    let Some(ev) = ev else { break };
                    self.on_network(ev).await;
                }
                peer_id = self.connect_rx.recv() => {
                    let Some(id) = peer_id else { break };
                    let peers = self.network.peers();
                    if let Some(p) = peers.iter().find(|p| p.id == id) {
                        let _ = self.network.connect(p).await;
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Input events
    // -----------------------------------------------------------------------

    async fn on_input(&mut self, event: InputEvent) {
        match self.state_machine.state() {
            State::Local | State::Transitioning => {
                if let InputEvent::MouseMove(pt) = event {
                    if let Some(crossed) = self.edge_detection.update(pt) {
                        let cmds = self.state_machine.handle(Event::EdgeCrossed(crossed));
                        self.execute_commands(cmds).await;
                    }
                }
                // Buttons and scroll stay local — not forwarded.
            }
            State::Remote => {
                match event {
                    InputEvent::MouseMove(pt) => {
                        let norm = self.screen_layout.map_to_remote(pt);
                        let _ = self
                            .network
                            .send(Message::MouseMove { x_norm: norm.x, y_norm: norm.y })
                            .await;
                    }
                    InputEvent::MouseButton { button, pressed } => {
                        let _ = self
                            .network
                            .send(Message::MouseButton { button, pressed })
                            .await;
                    }
                    InputEvent::Scroll { dx, dy } => {
                        let _ = self.network.send(Message::Scroll { dx, dy }).await;
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Network events
    // -----------------------------------------------------------------------

    async fn on_network(&mut self, event: NetworkEvent) {
        match event {
            NetworkEvent::StateChanged(ConnectionState::Connected) => {
                let cmds = self.state_machine.handle(Event::ConnectionEstablished);
                self.execute_commands(cmds).await;
                let _ = self.network.send(Message::ScreenInfo(self.local_dims)).await;
            }
            NetworkEvent::StateChanged(ConnectionState::Disconnected) => {
                let cmds = self.state_machine.handle(Event::ConnectionLost);
                self.execute_commands(cmds).await;
            }
            NetworkEvent::MessageReceived(msg) => {
                self.on_message(msg).await;
            }
            NetworkEvent::PeersUpdated(peers) => {
                if let Some(app) = &self.app_handle {
                    let _ = app.emit("peers-updated", peers);
                }
            }
            _ => {}
        }
    }

    async fn on_message(&mut self, msg: Message) {
        match msg {
            Message::MouseMove { x_norm, y_norm } => {
                #[cfg(target_os = "macos")]
                {
                    let pt = self
                        .screen_layout
                        .map_to_local(NormalizedPoint { x: x_norm, y: y_norm });
                    self.injector.inject_move(pt);
                }
            }
            Message::MouseButton { button, pressed } => {
                #[cfg(target_os = "macos")]
                self.injector.inject_button(button, pressed);
            }
            Message::Scroll { dx, dy } => {
                #[cfg(target_os = "macos")]
                self.injector.inject_scroll(dx, dy);
            }
            Message::TransitionIn { y_norm } => {
                let cmds = self
                    .state_machine
                    .handle(Event::TransitionInReceived { y_norm });
                self.execute_commands(cmds).await;
            }
            Message::Ack => {
                let cmds = self.state_machine.handle(Event::TransitionAcknowledged);
                self.execute_commands(cmds).await;
            }
            Message::ScreenInfo(dims) => {
                self.screen_layout
                    .configure(self.neighbor_side, self.local_dims, dims);
                if let Some(edge) = self.screen_layout.watched_edge() {
                    self.edge_detection
                        .configure(edge, self.local_dims, 2.0);
                }
            }
            Message::TransitionOut => {} // unused in v1
        }
    }

    // -----------------------------------------------------------------------
    // Command execution
    // -----------------------------------------------------------------------

    async fn execute_commands(&mut self, cmds: Vec<Command>) {
        for cmd in cmds {
            match cmd {
                Command::StartForwarding => {
                    #[cfg(target_os = "macos")]
                    {
                        self.capture.suppressing.store(true, Ordering::SeqCst);
                        self.injector.hide_cursor();
                    }
                }
                Command::StopForwarding => {
                    #[cfg(target_os = "macos")]
                    {
                        self.capture.suppressing.store(false, Ordering::SeqCst);
                        // Cursor visibility is restored by AcceptCursor (same Vec) or by
                        // the remote machine's AcceptCursor on its next transition.
                    }
                }
                Command::AcceptCursor { y_norm } => {
                    #[cfg(target_os = "macos")]
                    {
                        let entry_x_norm: f32 = match self.screen_layout.watched_edge() {
                            Some(Edge::Right) => 1.0,
                            Some(Edge::Left) => 0.0,
                            _ => 0.5, // top/bottom: center horizontally
                        };
                        let pt = self.screen_layout.map_to_local(NormalizedPoint {
                            x: entry_x_norm,
                            y: y_norm,
                        });
                        self.injector.show_cursor();
                        self.injector.inject_move(pt);
                    }
                }
                Command::Send(msg) => {
                    let _ = self.network.send(msg).await;
                }
            }
        }
    }
}

impl Drop for Coordinator {
    fn drop(&mut self) {
        self.stop();
    }
}

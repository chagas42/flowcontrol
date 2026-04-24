use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::engine::{
    edge_detection::EdgeDetectionImpl,
    fingerprint,
    protocol::Message,
    screen_layout::{
        neighbor_side_to_edge, opposite_edge, NeighborSide, NormalizedPoint, ScreenDimensions,
        ScreenLayoutImpl,
    },
    state_machine::{Command, Event, State, StateMachineImpl},
};
use crate::input::{InputCapture, InputEvent, InputInjector};
use crate::network::{ConnectionState, NetworkEvent, NetworkLayer, NetworkLayerImpl};

const PAIR_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Server,
    Client,
}

const LOCAL_OS: &str = if cfg!(target_os = "macos") {
    "mac"
} else if cfg!(target_os = "windows") {
    "win"
} else {
    "other"
};

/// Clamp an injected point so it lands on a valid on-screen pixel.
/// Without this, entries like `x_norm=1.0` produce `pt.x == width`, which is
/// one past the last valid pixel — macOS sometimes drops the cursor off-screen
/// in that case, making it look like the cursor vanished entirely.
fn clamp_to_screen(
    pt: crate::engine::screen_layout::Point,
    dims: ScreenDimensions,
) -> crate::engine::screen_layout::Point {
    let max_x = (dims.width.saturating_sub(1)) as f64;
    let max_y = (dims.height.saturating_sub(1)) as f64;
    crate::engine::screen_layout::Point {
        x: pt.x.clamp(0.0, max_x),
        y: pt.y.clamp(0.0, max_y),
    }
}

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

    /// Virtual cursor position used while in Remote state (suppressing=true).
    /// Accumulates hardware deltas because CGEventGetLocation is frozen when
    /// events are suppressed.
    virtual_pos: crate::engine::screen_layout::Point,

    /// Detects when virtual_pos crosses back to the local screen while in Remote state.
    return_edge_detection: EdgeDetectionImpl,

    /// y_norm of the edge crossing point, stored between on_input and execute_commands
    /// so StartForwarding can seed virtual_pos at the actual crossing position.
    pending_transition_y_norm: Option<f32>,

    /// Channel to receive manual connection triggers from frontend
    connect_rx: mpsc::Receiver<String>,
    app_handle: Option<AppHandle>,

    /// Flips to true the first time we reach `ConnectionState::Connected`.
    /// Used by `emit_status()` to distinguish "never connected" (`Stopped`)
    /// from "was connected, now lost it" (`Disconnected`).
    had_connection: bool,

    /// Whether this peer is the mouse-owning server or the display-only client.
    role: Role,
    /// Human-readable device name sent in `PairRequest` and displayed to the peer.
    device_name: String,
    /// 16-byte nonce generated per-connection; sent in our `PairRequest`.
    local_nonce: [u8; 16],
    /// Peer's nonce, populated when we receive their `PairRequest`.
    peer_nonce: Option<[u8; 16]>,
    /// Peer's device name + os, populated when we receive their `PairRequest`.
    peer_name: Option<String>,
    peer_os: Option<String>,

    /// User/peer accept/decline channel.
    pair_response_rx: mpsc::Receiver<bool>,
    /// 30s auto-decline timer for the Pairing state.
    pair_timeout_tx: mpsc::Sender<()>,
    pair_timeout_rx: mpsc::Receiver<()>,
    pair_timer_handle: Option<JoinHandle<()>>,

    /// Pause/Resume: when true, keep TCP up but drop every outbound
    /// MouseMove/Button/Scroll and every inbound injected event. Does NOT
    /// affect pair messages or TransitionIn/Ack (the FSM stays consistent).
    suppressed: bool,
    /// Pause/Resume channel: tray + frontend push `true` for pause,
    /// `false` for resume. Event loop drains it next tick.
    pause_rx: mpsc::Receiver<bool>,
}

impl Coordinator {
    #[allow(clippy::too_many_arguments)]
    fn new_inner(
        state_machine: StateMachineImpl,
        role: Role,
        device_name: String,
        local_dims: ScreenDimensions,
        side: NeighborSide,
        app_handle: Option<AppHandle>,
        connect_rx: mpsc::Receiver<String>,
        pair_response_rx: mpsc::Receiver<bool>,
        pause_rx: mpsc::Receiver<bool>,
    ) -> Self {
        let (input_tx, input_rx) = mpsc::channel::<InputEvent>(256);
        let (net_tx, network_rx) = mpsc::channel::<NetworkEvent>(32);
        let (pair_timeout_tx, pair_timeout_rx) = mpsc::channel::<()>(1);

        let mut edge_detection = EdgeDetectionImpl::new();
        edge_detection.configure(neighbor_side_to_edge(side), local_dims, 10.0);

        let local_nonce = {
            use rand::RngCore;
            let mut n = [0u8; 16];
            rand::rngs::OsRng.fill_bytes(&mut n);
            n
        };

        Self {
            state_machine,
            screen_layout: ScreenLayoutImpl::new(),
            edge_detection,
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
            virtual_pos: crate::engine::screen_layout::Point { x: 0.0, y: 0.0 },
            return_edge_detection: EdgeDetectionImpl::new(),
            pending_transition_y_norm: None,
            connect_rx,
            app_handle,
            had_connection: false,

            role,
            device_name,
            local_nonce,
            peer_nonce: None,
            peer_name: None,
            peer_os: None,

            pair_response_rx,
            pair_timeout_tx,
            pair_timeout_rx,
            pair_timer_handle: None,

            suppressed: false,
            pause_rx,
        }
    }

    /// Coordinator seeded as the server (physical mouse machine).
    /// State machine starts in `Local`.
    #[allow(clippy::too_many_arguments)]
    pub fn new_server(
        device_name: String,
        local_dims: ScreenDimensions,
        side: NeighborSide,
        app_handle: Option<AppHandle>,
        connect_rx: mpsc::Receiver<String>,
        pair_response_rx: mpsc::Receiver<bool>,
        pause_rx: mpsc::Receiver<bool>,
    ) -> Self {
        Self::new_inner(
            StateMachineImpl::new(),
            Role::Server,
            device_name,
            local_dims,
            side,
            app_handle,
            connect_rx,
            pair_response_rx,
            pause_rx,
        )
    }

    /// Coordinator seeded as the client (display-only machine).
    /// State machine starts in `Remote`.
    #[allow(clippy::too_many_arguments)]
    pub fn new_client(
        device_name: String,
        local_dims: ScreenDimensions,
        side: NeighborSide,
        app_handle: Option<AppHandle>,
        connect_rx: mpsc::Receiver<String>,
        pair_response_rx: mpsc::Receiver<bool>,
        pause_rx: mpsc::Receiver<bool>,
    ) -> Self {
        Self::new_inner(
            StateMachineImpl::new_as_client(),
            Role::Client,
            device_name,
            local_dims,
            side,
            app_handle,
            connect_rx,
            pair_response_rx,
            pause_rx,
        )
    }

    // -----------------------------------------------------------------------
    // Entry points
    // -----------------------------------------------------------------------

    /// Start as the server: advertise via mDNS, capture input, run event loop.
    pub async fn run_as_server(&mut self, name: &str) -> Result<(), NetworkError> {
        eprintln!(
            "[fc] START server name={name} local={}x{} side={:?}",
            self.local_dims.width, self.local_dims.height, self.neighbor_side
        );
        self.network.start_server(name).await?;
        self.start_capture();
        self.emit_status();
        self.event_loop().await;
        Ok(())
    }

    /// Start as the client: browse mDNS, run event loop.
    /// The client only injects events received from the server — no local capture needed.
    /// Call `network.connect(peer)` separately after peers are discovered.
    pub async fn run_as_client(&mut self) -> Result<(), NetworkError> {
        eprintln!(
            "[fc] START client name={} local={}x{} side={:?}",
            self.device_name, self.local_dims.width, self.local_dims.height, self.neighbor_side
        );
        #[cfg(target_os = "macos")]
        {
            use crate::input::{InputCapture, PermissionStatus};
            let status = crate::input::macos::MacOSCapture::new().permission_status();
            eprintln!(
                "[fc]   Accessibility on this (client) Mac: {:?} {}",
                status,
                if status == PermissionStatus::Granted {
                    "✓"
                } else {
                    "✗ inject_move will silently fail until granted"
                }
            );
        }
        self.network.start_client().await?;
        self.emit_status();
        self.event_loop().await;
        Ok(())
    }

    /// Non-fatal capture start. If Accessibility permission is not granted,
    /// emits `permission-required` to the frontend and continues — network
    /// events (injection, coordination) still work; only edge-detection and
    /// local button/scroll forwarding are affected until permission is granted.
    fn start_capture(&mut self) {
        #[cfg(target_os = "macos")]
        {
            if let Err(e) = self.capture.start(self.input_tx.clone()) {
                eprintln!("[coordinator] capture failed: {e}");
                if let Some(app) = &self.app_handle {
                    let _ = app.emit("permission-required", ());
                }
            }
        }
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
                    // Try as mDNS peer ID first, then as bare IP (direct connect).
                    let peers = self.network.peers();
                    if let Some(p) = peers.iter().find(|p| p.id == id) {
                        let _ = self.network.connect(p).await;
                    } else {
                        // Accept "192.168.x.x" or "192.168.x.x:7878"
                        use std::net::SocketAddr;
                        let addr_str = if id.contains(':') {
                            id.clone()
                        } else {
                            format!("{}:{}", id, 7878)
                        };
                        if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                            let _ = self.network.connect_direct(addr).await;
                        }
                    }
                }
                resp = self.pair_response_rx.recv() => {
                    let Some(accept) = resp else { break };
                    self.on_pair_response(accept).await;
                }
                timeout = self.pair_timeout_rx.recv() => {
                    if timeout.is_none() { break; }
                    self.on_pair_timeout().await;
                }
                paused = self.pause_rx.recv() => {
                    let Some(paused) = paused else { break };
                    self.set_paused(paused);
                }
            }
        }
    }

    fn set_paused(&mut self, paused: bool) {
        if self.suppressed == paused {
            return;
        }
        self.suppressed = paused;
        #[cfg(target_os = "macos")]
        {
            // While paused, also stop suppressing hardware events at the
            // OS layer so the user can keep working on this Mac normally.
            // Resume restores suppression iff the FSM is in Remote again.
            if paused {
                self.capture.suppressing.store(false, Ordering::SeqCst);
                self.injector.show_cursor();
            } else if self.state_machine.state() == State::Remote {
                self.capture.suppressing.store(true, Ordering::SeqCst);
                self.injector.hide_cursor();
            }
        }
        self.emit_status();
    }

    // -----------------------------------------------------------------------
    // Input events
    // -----------------------------------------------------------------------

    async fn on_input(&mut self, event: InputEvent) {
        // Pause gate: keep TCP up but don't act on any input events.
        // Edge detection is also disabled — the cursor can't cross while paused.
        if self.suppressed {
            return;
        }
        // Refuse to fire edge detection / forwarding until there's an actually
        // connected, post-pair peer. Without this guard, a solo server would
        // go to Remote on the first edge-cross, hide its own cursor, and spam
        // MouseMove into the void (also with garbage norms because
        // screen_layout isn't configured yet).
        let is_post_pair_connected = self.network.state() == ConnectionState::Connected
            && self.state_machine.state() != State::Pairing;
        if !is_post_pair_connected
            && matches!(self.state_machine.state(), State::Local | State::Remote)
        {
            // Absorb the event silently; we're not paired, edge detection is off.
            return;
        }
        match self.state_machine.state() {
            // Waiting for the user to approve the pair — drop every input.
            State::Pairing => {}
            State::Local => {
                if let InputEvent::MouseMove(pt) = event {
                    if let Some(crossed) = self.edge_detection.update(pt) {
                        eprintln!(
                            "[fc] EDGE CROSSED edge={:?} y_norm={:.3}",
                            crossed.edge, crossed.y_norm
                        );
                        self.pending_transition_y_norm = Some(crossed.y_norm);
                        let cmds = self.state_machine.handle(Event::EdgeCrossed(crossed));
                        self.execute_commands(cmds).await;
                    }
                }
                // Buttons and scroll stay local — not forwarded.
            }
            State::Transitioning | State::ReturnTransitioning => {
                // Waiting for Ack — pass through, no edge detection, no forwarding.
            }
            State::Remote => {
                match event {
                    InputEvent::MouseDelta { dx, dy, button } => {
                        // While suppressing, CGEventGetLocation is frozen at the edge.
                        // Accumulate hardware deltas into virtual_pos instead.
                        let w = self.local_dims.width as f64;
                        let h = self.local_dims.height as f64;
                        self.virtual_pos.x = (self.virtual_pos.x + dx).clamp(0.0, w - 1.0);
                        self.virtual_pos.y = (self.virtual_pos.y + dy).clamp(0.0, h - 1.0);

                        // Check if the cursor has crossed back to the local screen.
                        if let Some(crossed) = self.return_edge_detection.update(self.virtual_pos) {
                            let cmds = self.state_machine.handle(Event::CursorReturnedToLocal {
                                y_norm: crossed.y_norm,
                            });
                            self.execute_commands(cmds).await;
                            // Skip sending MouseMove this tick — cursor is returning.
                            return;
                        }

                        let norm = self.screen_layout.map_to_remote(self.virtual_pos);
                        static TX_COUNT: std::sync::atomic::AtomicU32 =
                            std::sync::atomic::AtomicU32::new(0);
                        let n = TX_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if n.is_multiple_of(30) {
                            eprintln!(
                                "[fc] TX MouseMove #{n} virtual=({:.0},{:.0}) norm=({:.3},{:.3})",
                                self.virtual_pos.x, self.virtual_pos.y, norm.x, norm.y
                            );
                        }
                        let _ = self
                            .network
                            .send(Message::MouseMove {
                                x_norm: norm.x,
                                y_norm: norm.y,
                                button,
                            })
                            .await;
                    }
                    InputEvent::MouseMove(_) => {
                        // Absolute events arrive briefly before suppressing kicks in.
                        // Ignore them in Remote state — virtual_pos is the source of truth.
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
                self.had_connection = true;
                // State machine moves to Pairing. Fire our PairRequest and arm
                // the 30s auto-decline timer. ScreenInfo waits until pairing
                // is accepted (see on_pair_response).
                let cmds = self.state_machine.handle(Event::ConnectionEstablished);
                self.execute_commands(cmds).await;
                let _ = self
                    .network
                    .send(Message::PairRequest {
                        nonce: self.local_nonce,
                        device_name: self.device_name.clone(),
                        os: LOCAL_OS.to_string(),
                    })
                    .await;
                self.arm_pair_timer();
                self.emit_status();
            }
            NetworkEvent::StateChanged(ConnectionState::Disconnected) => {
                self.cancel_pair_timer();
                self.peer_nonce = None;
                self.peer_name = None;
                self.peer_os = None;
                let cmds = self.state_machine.handle(Event::ConnectionLost);
                self.execute_commands(cmds).await;
                self.emit_status();
            }
            NetworkEvent::StateChanged(_) => {
                // Browsing / Advertising / Connecting — still "Searching" at the UI layer.
                self.emit_status();
            }
            NetworkEvent::MessageReceived(msg) => {
                self.on_message(msg).await;
            }
            NetworkEvent::PeersUpdated(peers) => {
                if let Some(app) = &self.app_handle {
                    let _ = app.emit("peers-updated", &peers);
                    let views: Vec<crate::tray::PeerView> = peers
                        .into_iter()
                        .map(|p| crate::tray::PeerView {
                            name: p.name,
                            online: true,
                        })
                        .collect();
                    let _ = crate::tray::rebuild_menu(app, &views, self.current_status_str());
                }
            }
        }
    }

    /// Returns the current status string (same logic as `emit_status`) for
    /// consumers like the tray menu that need to read it synchronously.
    fn current_status_str(&self) -> &'static str {
        use crate::engine::state_machine::State;
        // Paused takes priority while we have an active connection — the TCP
        // session is live but we're silently dropping input.
        if self.suppressed && self.network.state() == ConnectionState::Connected {
            return "Paused";
        }
        // Remote + Local only "mean" anything once the TCP is up AND we're
        // past Pairing. The client-side state_machine defaults to Remote at
        // construction, so this guard prevents the UI from claiming "Remote"
        // (and the peer-card from faking a "paired · 4ms" label) before any
        // handshake has actually happened.
        match (self.state_machine.state(), self.network.state()) {
            (State::Pairing, _) => "Searching",
            (State::Remote | State::ReturnTransitioning, ConnectionState::Connected) => "Remote",
            (State::Local | State::Transitioning, ConnectionState::Connected) => "Connected",
            (_, ConnectionState::Browsing)
            | (_, ConnectionState::Advertising)
            | (_, ConnectionState::Connecting) => "Searching",
            (_, ConnectionState::Disconnected) if self.had_connection => "Disconnected",
            _ => "Stopped",
        }
    }

    async fn on_message(&mut self, msg: Message) {
        // Pair messages are valid in any state; coordinate messages are blocked
        // while we're still waiting for the user to accept the pair.
        let is_coord_msg = matches!(
            msg,
            Message::MouseMove { .. }
                | Message::MouseButton { .. }
                | Message::Scroll { .. }
                | Message::TransitionIn { .. }
                | Message::TransitionOut
                | Message::Ack
                | Message::ScreenInfo(_)
        );
        if is_coord_msg && self.state_machine.state() == State::Pairing {
            eprintln!(
                "[fc] DROPPED {:?} — state is still Pairing (peer hasn't accepted yet?)",
                std::mem::discriminant(&msg)
            );
            return;
        }

        match msg {
            Message::MouseMove {
                x_norm,
                y_norm,
                button,
            } => {
                #[cfg(target_os = "macos")]
                {
                    let pt = self.screen_layout.map_to_local(NormalizedPoint {
                        x: x_norm,
                        y: y_norm,
                    });
                    let pt = clamp_to_screen(pt, self.local_dims);
                    // Sample ~1 in 30 so the terminal isn't flooded at 120Hz.
                    static MM_COUNT: std::sync::atomic::AtomicU32 =
                        std::sync::atomic::AtomicU32::new(0);
                    let n = MM_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if n.is_multiple_of(30) {
                        eprintln!(
                            "[fc] MouseMove #{n} norm=({x_norm:.3},{y_norm:.3}) → pt=({:.1},{:.1})",
                            pt.x, pt.y
                        );
                    }
                    self.injector.inject_move(pt, button);
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = (x_norm, y_norm, button);
                }
            }
            Message::MouseButton { button, pressed } => {
                #[cfg(target_os = "macos")]
                self.injector.inject_button(button, pressed);
                #[cfg(not(target_os = "macos"))]
                let _ = (button, pressed);
            }
            Message::Scroll { dx, dy } => {
                #[cfg(target_os = "macos")]
                self.injector.inject_scroll(dx, dy);
                #[cfg(not(target_os = "macos"))]
                let _ = (dx, dy);
            }
            Message::TransitionIn { y_norm } => {
                eprintln!(
                    "[fc] RX TransitionIn y_norm={y_norm:.3} state_before={:?}",
                    self.state_machine.state()
                );
                let cmds = self
                    .state_machine
                    .handle(Event::TransitionInReceived { y_norm });
                eprintln!(
                    "[fc]    state_after={:?} cmds={}",
                    self.state_machine.state(),
                    cmds.len()
                );
                self.execute_commands(cmds).await;
            }
            Message::Ack => {
                eprintln!("[fc] RX Ack state_before={:?}", self.state_machine.state());
                let cmds = self.state_machine.handle(Event::TransitionAcknowledged);
                self.execute_commands(cmds).await;
            }
            Message::ScreenInfo(dims) => {
                eprintln!(
                    "[fc] RX ScreenInfo peer_dims={}x{} local_dims={}x{} side={:?}",
                    dims.width,
                    dims.height,
                    self.local_dims.width,
                    self.local_dims.height,
                    self.neighbor_side
                );
                self.screen_layout
                    .configure(self.neighbor_side, self.local_dims, dims);
                if let Some(edge) = self.screen_layout.watched_edge() {
                    self.edge_detection.configure(edge, self.local_dims, 10.0);
                }
            }
            Message::TransitionOut => {} // unused in v1
            Message::PairRequest {
                nonce,
                device_name,
                os,
            } => {
                eprintln!("[fc] RX PairRequest from {device_name} ({os})");
                self.peer_nonce = Some(nonce);
                self.peer_name = Some(device_name.clone());
                self.peer_os = Some(os.clone());
                let fp_str = self.derive_fingerprint_string();
                if let Some(app) = &self.app_handle {
                    let _ = app.emit(
                        "pair-incoming",
                        serde_json::json!({
                            "peer_name": device_name,
                            "fingerprint": fp_str,
                            "os": os,
                        }),
                    );
                }
            }
            Message::PairAccept => {
                eprintln!(
                    "[fc] RX PairAccept — auto-accepting locally (state_before={:?})",
                    self.state_machine.state()
                );
                self.cancel_pair_timer();
                let cmds = self.state_machine.handle(Event::PairAccepted);
                self.execute_commands(cmds).await;
                // Both sides publish their own dims so the peer's ScreenLayout
                // + EdgeDetection can be configured. Idempotent if the peer
                // already sent theirs.
                let _ = self
                    .network
                    .send(Message::ScreenInfo(self.local_dims))
                    .await;
                if let Some(app) = &self.app_handle {
                    let _ = app.emit("pair-resolved", ());
                }
                self.emit_status();
            }
            Message::PairDecline => {
                self.cancel_pair_timer();
                let cmds = self.state_machine.handle(Event::PairDeclined);
                self.execute_commands(cmds).await;
                if let Some(app) = &self.app_handle {
                    let _ = app.emit("pair-resolved", ());
                }
                // Tear the connection down — user on the other side said no.
                self.network.stop();
                self.emit_status();
            }
        }
    }

    /// Derives the display fingerprint from our nonce + peer's nonce.
    /// Ordering is canonical: client nonce first, server nonce second, so
    /// both peers produce the same string regardless of role.
    fn derive_fingerprint_string(&self) -> String {
        let Some(peer) = self.peer_nonce else {
            return String::new();
        };
        let (client_nonce, server_nonce) = match self.role {
            Role::Server => (&peer, &self.local_nonce),
            Role::Client => (&self.local_nonce, &peer),
        };
        let fp = fingerprint::derive(client_nonce, server_nonce);
        fingerprint::render(&fp)
    }

    /// Called from the event loop when the frontend (or tray) dispatches an
    /// accept/decline. `true` = accept, `false` = decline.
    async fn on_pair_response(&mut self, accept: bool) {
        if self.state_machine.state() != State::Pairing {
            return;
        }
        self.cancel_pair_timer();
        if accept {
            let _ = self.network.send(Message::PairAccept).await;
            let cmds = self.state_machine.handle(Event::PairAccepted);
            self.execute_commands(cmds).await;
            // Both sides publish their own dims — see the PairAccept message
            // branch for the symmetric case.
            let _ = self
                .network
                .send(Message::ScreenInfo(self.local_dims))
                .await;
        } else {
            let _ = self.network.send(Message::PairDecline).await;
            let cmds = self.state_machine.handle(Event::PairDeclined);
            self.execute_commands(cmds).await;
            self.network.stop();
        }
        if let Some(app) = &self.app_handle {
            let _ = app.emit("pair-resolved", ());
        }
        self.emit_status();
    }

    /// 30s elapsed with no accept/decline — auto-decline.
    async fn on_pair_timeout(&mut self) {
        if self.state_machine.state() != State::Pairing {
            return;
        }
        let _ = self.network.send(Message::PairDecline).await;
        let cmds = self.state_machine.handle(Event::PairDeclined);
        self.execute_commands(cmds).await;
        if let Some(app) = &self.app_handle {
            let _ = app.emit("pair-timeout", ());
            let _ = app.emit("pair-resolved", ());
        }
        self.network.stop();
        self.emit_status();
    }

    fn arm_pair_timer(&mut self) {
        self.cancel_pair_timer();
        let tx = self.pair_timeout_tx.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(PAIR_TIMEOUT).await;
            let _ = tx.try_send(());
        });
        self.pair_timer_handle = Some(handle);
    }

    fn cancel_pair_timer(&mut self) {
        if let Some(h) = self.pair_timer_handle.take() {
            h.abort();
        }
    }

    /// Fires exactly once per edge crossing (tied to StartForwarding /
    /// ReturnCursorToLocal commands). The frontend turns this into a toast.
    fn emit_cursor_crossed(&self, direction: &str) {
        let Some(app) = &self.app_handle else { return };
        let _ = app.emit(
            "cursor-crossed",
            serde_json::json!({
                "direction": direction,
                "peer_name": self.peer_name.clone().unwrap_or_default(),
            }),
        );
    }

    // -----------------------------------------------------------------------
    // Command execution
    // -----------------------------------------------------------------------

    /// Maps (state-machine state × network connection state) → one of the six
    /// status strings the frontend listens for. Called whenever either axis
    /// changes; safe to call more than once per event — frontend receives a
    /// duplicate emit at worst.
    fn emit_status(&self) {
        let status = self.current_status_str();
        if let Some(app) = &self.app_handle {
            let _ = app.emit("status-changed", status);
            // Keep the tray in sync with the same status line and current peers.
            let views: Vec<crate::tray::PeerView> = self
                .network
                .peers()
                .into_iter()
                .map(|p| crate::tray::PeerView {
                    name: p.name,
                    online: true,
                })
                .collect();
            let _ = crate::tray::rebuild_menu(app, &views, status);
        }
    }

    async fn execute_commands(&mut self, cmds: Vec<Command>) {
        let had_commands = !cmds.is_empty();
        for cmd in cmds {
            match cmd {
                Command::StartForwarding => {
                    #[cfg(target_os = "macos")]
                    {
                        // Seed virtual_pos at the actual edge-crossing pixel so the first
                        // MouseMove sent to the client matches the cursor's true position.
                        let y_norm = self.pending_transition_y_norm.take().unwrap_or(0.5);
                        let edge_x = match self.neighbor_side {
                            NeighborSide::Right => self.local_dims.width as f64 - 1.0,
                            NeighborSide::Left => 0.0,
                            NeighborSide::Top => self.local_dims.width as f64 / 2.0,
                            NeighborSide::Bottom => self.local_dims.width as f64 / 2.0,
                        };
                        self.virtual_pos = crate::engine::screen_layout::Point {
                            x: edge_x,
                            y: y_norm as f64 * self.local_dims.height as f64,
                        };
                        // Configure return edge detection for when the cursor comes back.
                        self.return_edge_detection.configure(
                            opposite_edge(self.neighbor_side),
                            self.local_dims,
                            10.0,
                        );
                        self.capture.suppressing.store(true, Ordering::SeqCst);
                        self.injector.hide_cursor();
                    }
                    self.emit_cursor_crossed("to_remote");
                }
                Command::StopForwarding => {
                    #[cfg(target_os = "macos")]
                    {
                        self.capture.suppressing.store(false, Ordering::SeqCst);
                        self.capture.grace_frames.store(3, Ordering::SeqCst);
                        // Cursor visibility is restored by AcceptCursor (same Vec) or by
                        // the remote machine's AcceptCursor on its next transition.
                    }
                }
                Command::AcceptCursor { y_norm } => {
                    #[cfg(target_os = "macos")]
                    {
                        let watched = self.screen_layout.watched_edge();
                        let entry_x_norm: f32 = match watched {
                            Some(Edge::Right) => 1.0,
                            Some(Edge::Left) => 0.0,
                            _ => 0.5, // top/bottom: center horizontally
                        };
                        let pt = self.screen_layout.map_to_local(NormalizedPoint {
                            x: entry_x_norm,
                            y: y_norm,
                        });
                        let pt = clamp_to_screen(pt, self.local_dims);
                        eprintln!(
                            "[fc] AcceptCursor watched={:?} entry_x_norm={} y_norm={:.3} pt=({:.1},{:.1}) local_dims={}x{}",
                            watched,
                            entry_x_norm,
                            y_norm,
                            pt.x,
                            pt.y,
                            self.local_dims.width,
                            self.local_dims.height
                        );
                        self.injector.show_cursor();
                        self.injector.inject_move(pt, None);
                    }
                    // Reset edge_detection so the next edge-cross on this side
                    // isn't eaten by a stale "was_at_edge = true".
                    self.edge_detection.reset();
                }
                Command::ReturnCursorToLocal { y_norm } => {
                    #[cfg(target_os = "macos")]
                    {
                        // Stop suppression — hardware events flow through again.
                        self.capture.suppressing.store(false, Ordering::SeqCst);
                        self.capture.grace_frames.store(3, Ordering::SeqCst);
                        // Cursor re-enters at the local screen edge opposite to the neighbor.
                        // e.g. Right neighbor → cursor returns at the Right edge (x_norm = 1.0).
                        let entry_x_norm: f32 = match self.neighbor_side {
                            NeighborSide::Right => 1.0,
                            NeighborSide::Left => 0.0,
                            NeighborSide::Top => 0.5,
                            NeighborSide::Bottom => 0.5,
                        };
                        let pt = self.screen_layout.map_to_local(NormalizedPoint {
                            x: entry_x_norm,
                            y: y_norm,
                        });
                        let pt = clamp_to_screen(pt, self.local_dims);
                        self.injector.show_cursor();
                        self.injector.inject_move(pt, None);
                    }
                    #[cfg(not(target_os = "macos"))]
                    let _ = y_norm;
                    // After returning to Local, clear any stale "was_at_edge" so
                    // the next edge crossing fires. Without this, bug: cursor
                    // goes out once and never crosses again.
                    self.edge_detection.reset();
                    self.emit_cursor_crossed("to_local");
                }
                Command::Send(msg) => {
                    let _ = self.network.send(msg).await;
                }
            }
        }
        if had_commands {
            self.emit_status();
        }
    }
}

impl Drop for Coordinator {
    fn drop(&mut self) {
        self.stop();
    }
}

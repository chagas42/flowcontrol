# FlowControl — Backend Implementation Plan

**Audience:** Claude Code running inside the FlowControl Tauri repo.
**Goal:** Implement every backend capability required by the frontend port delivered in `svelte-port/`. Ship in the order below. Each phase leaves `cargo check` green and the app runnable.

**Conventions:**
- All event payloads are JSON-serializable via `serde`.
- All Tauri commands return `Result<T, String>` so the frontend can `await invoke(...)` and catch errors.
- Wire-protocol changes are additive (new `Message` variants only — never rename/reorder existing ones).
- Emit events through the `AppHandle` stashed on `AppState`. Do not create new handles.

---

## Phase 4a — Richer status events (30 min)

**Problem.** Frontend expects 6 status values, backend only emits 3.

**Files to edit**
- `src-tauri/src/coordinator.rs`

**What to do**
1. Add a helper at the bottom of the `Coordinator` impl:
   ```rust
   fn emit_status(&self) {
       use engine::state_machine::State;
       use network::ConnectionState;
       let status = match (self.sm.state(), self.net.connection_state()) {
           (State::Local,  ConnectionState::Connected)    => "Connected",
           (State::Remote, _)                             => "Remote",
           (_, ConnectionState::Browsing)                 => "Searching",
           (_, ConnectionState::Disconnected) if self.had_connection => "Disconnected",
           (_, ConnectionState::Paused)                   => "Paused",
           _                                              => "Stopped",
       };
       if let Some(h) = &self.app_handle {
           let _ = h.emit("status-changed", status);
       }
   }
   ```
2. Add `had_connection: bool` to `Coordinator`; set it `true` the first time we reach `Connected`. This lets us distinguish "never connected" (`Stopped`) from "was connected, now lost it" (`Disconnected`).
3. Call `self.emit_status()` after every state machine transition and every network state change — anywhere we currently emit the old status string.
4. Delete the old 3-string emit paths.

**Acceptance**
- `cargo test -p coordinator` passes.
- Launching the app with no peers emits `"Searching"`.
- Stopping emits `"Stopped"`.
- Killing the peer mid-connection emits `"Disconnected"`, not `"Stopped"`.

---

## Phase 4b — Pair request + fingerprint (4-5h)

**Problem.** Current `connect_to_peer` opens TCP and starts exchanging coordinates immediately. Frontend expects a human-approved pair handshake with a 4-group fingerprint displayed on both sides.

### 4b.1 — Protocol (additive)

**Files**
- `src-tauri/src/engine/protocol.rs`

**Add** (do not remove existing variants):
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    // ...existing variants...
    PairRequest { fingerprint: [u8; 5], device_name: String },
    PairAccept,
    PairDecline,
}
```
Why 5 bytes: renders to four base32 groups with a check char (e.g. `84-ZX-19-PF`). See 4b.3.

### 4b.2 — State machine gate

**Files**
- `src-tauri/src/engine/state_machine.rs`
- `src-tauri/src/coordinator.rs`

Add a `Pairing` sub-state that precedes `Local`:
```rust
pub enum State { Pairing, Local, Remote }
```
`Pairing` blocks coordinate emission/consumption. Transitions:
- `Pairing + incoming PairAccept` → `Local` (outbound side) or `Remote` (inbound side, if we want to default to handing off).
- `Pairing + incoming PairDecline` → drop connection, back to `Disconnected`.
- Timeout 30s in `Pairing` → auto-decline.

In `Coordinator::on_tcp_connected`, immediately send a `PairRequest` with a freshly generated fingerprint (4b.3) and enter `Pairing`. Do NOT start reading `MouseMove` messages until the state machine leaves `Pairing`.

### 4b.3 — Fingerprint generation

**New file:** `src-tauri/src/engine/fingerprint.rs`

```rust
use blake3::Hasher;

/// Derive a 5-byte fingerprint from the TCP connection's shared nonces.
/// Both peers MUST call this with the same (client_nonce, server_nonce)
/// ordering to produce the same fingerprint.
pub fn derive(client_nonce: &[u8; 16], server_nonce: &[u8; 16]) -> [u8; 5] {
    let mut h = Hasher::new();
    h.update(b"flowcontrol-pair-v1");
    h.update(client_nonce);
    h.update(server_nonce);
    let out = h.finalize();
    let mut fp = [0u8; 5];
    fp.copy_from_slice(&out.as_bytes()[..5]);
    fp
}

/// Render "84-ZX-19-PF" style string for the UI.
pub fn render(fp: &[u8; 5]) -> String {
    const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ"; // Crockford, no I/L/O/0/1
    let mut out = String::with_capacity(11);
    for (i, chunk) in fp.chunks(2).enumerate() {
        if i > 0 { out.push('-'); }
        let n = if chunk.len() == 2 {
            ((chunk[0] as u16) << 8) | chunk[1] as u16
        } else {
            (chunk[0] as u16) << 8
        };
        out.push(ALPHABET[((n >> 11) & 0x1F) as usize] as char);
        out.push(ALPHABET[((n >> 6)  & 0x1F) as usize] as char);
        if chunk.len() == 2 {
            out.push(ALPHABET[((n >> 1) & 0x1F) as usize] as char);
        }
    }
    out
}
```

Add `blake3 = "1"` to `Cargo.toml`. Nonces: generate 16 random bytes on both sides as the first step of the TCP handshake (exchange before any `Message`). Use `rand::rngs::OsRng`.

### 4b.4 — Tauri commands

**Files**
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs` (register in `invoke_handler`)

```rust
#[tauri::command]
pub async fn pair_accept(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.coordinator.lock().await.send_pair_response(true).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pair_decline(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.coordinator.lock().await.send_pair_response(false).await
        .map_err(|e| e.to_string())
}
```

In `Coordinator`, keep the pending outbound `TcpStream` reachable until the user answers. On `pair_accept`, send `Message::PairAccept` and transition to `Local`. On `pair_decline`, send `Message::PairDecline` and drop the connection.

### 4b.5 — Event emission

On receiving `Message::PairRequest` from the other side:
```rust
app_handle.emit("pair-incoming", serde_json::json!({
    "peer_name": device_name,
    "fingerprint": render(&fp),   // human string
    "os": detect_peer_os(),        // "mac" | "win" — infer from device_name or send explicitly
}))?;
```

### 4b.6 — Outbound flow update

- `connect_to_peer` now returns after sending `PairRequest`. Frontend is responsible for showing the modal on the **other** side; this side waits for `PairAccept`/`PairDecline` and emits its own `pair-resolved` event for local UI.

**Acceptance**
- Two devices connect → both receive `pair-incoming` with the **same** fingerprint string.
- Accepting on one side while the other declines → both return to `Disconnected`.
- Timeout of 30s without response → `Disconnected` + emit `"pair-timeout"` event.
- Mouse coordinates never flow during `Pairing`.

---

## Phase 5 — Tray icon menu (3-4h)

**Problem.** `tauri.conf.json` already declares the tray icon but `lib.rs` never builds a menu. Frontend expects quick actions accessible from the menu bar / system tray.

**Files**
- `src-tauri/src/lib.rs`
- `src-tauri/src/tray.rs` (new)
- `src-tauri/icons/tray-template.png` (new — monochrome template for macOS)

### 5.1 — Build initial menu in `setup()`

```rust
// tray.rs
use tauri::{AppHandle, Manager, Runtime};
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};

pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open = MenuItemBuilder::with_id("open", "Open FlowControl").build(app)?;
    let pause = MenuItemBuilder::with_id("pause", "Pause sharing").build(app)?;
    let about = MenuItemBuilder::with_id("about", "About").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&open, &pause,
                 &PredefinedMenuItem::separator(app)?,
                 &about, &quit])
        .build()?;

    let _tray = TrayIconBuilder::with_id("fc-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .icon_as_template(true)  // macOS monochrome
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open"  => { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); } }
            "pause" => { let _ = app.emit("tray-action", "pause"); }
            "about" => { let _ = app.emit("tray-action", "about"); }
            "quit"  => { app.exit(0); }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); }
            }
        })
        .build(app)?;
    Ok(())
}
```

Call `tray::build_tray(&app.handle())?` from `lib::run`'s `setup` closure.

### 5.2 — Dynamic peer list

Whenever `peers-updated` fires in the coordinator, rebuild the tray menu with a submenu listing each peer and a status dot (emoji fallback: 🟢 ready, 🔴 offline, 🟠 paused). Store the tray handle on `AppState`:
```rust
pub struct AppState {
    // ...
    pub tray: Mutex<Option<TrayIcon>>,
}
```

Rebuild helper:
```rust
pub fn rebuild_menu(app: &AppHandle, peers: &[Peer], status: &str) -> tauri::Result<Menu> { /* ... */ }
```

### 5.3 — Template icon

Create `src-tauri/icons/tray-template.png` — 44×44, monochrome with alpha, per Apple HIG. Reference it only when building for macOS:
```rust
#[cfg(target_os = "macos")]
let icon = Image::from_path("icons/tray-template.png")?;
```

### 5.4 — Tray action plumbing

Frontend already consumes `listen('tray-action', ...)`; the emit is above. Frontend navigates to the About screen or calls `pause_sharing()` accordingly.

**Acceptance**
- Click tray icon → window shows and focuses.
- "Quit" stops the coordinator (call `stop_coordinator` before `app.exit`).
- Menu updates live when peers appear/disappear.
- Template icon is monochrome on macOS light and dark menu bars.

---

## Phase 6a — Pause / Resume sharing (1h)

**Problem.** Frontend has a pause banner and tray action but no backend command.

**Files**
- `src-tauri/src/commands.rs`
- `src-tauri/src/coordinator.rs`
- `src-tauri/src/network/mod.rs` (add `Paused` variant to `ConnectionState` if needed)

**What to do**
1. Add `suppressed: bool` to `Coordinator` (distinct from `suppressing`, which is input-OS-level). When `true`, drop every outbound `MouseMove` without tearing down TCP.
2. Commands:
   ```rust
   #[tauri::command]
   pub async fn pause_sharing(state: State<'_, Arc<AppState>>) -> Result<(), String> {
       state.coordinator.lock().await.set_paused(true).await; Ok(())
   }
   #[tauri::command]
   pub async fn resume_sharing(state: State<'_, Arc<AppState>>) -> Result<(), String> {
       state.coordinator.lock().await.set_paused(false).await; Ok(())
   }
   ```
3. `set_paused` emits `status-changed` with `"Paused"` / previous status.

**Acceptance**
- Pause while in `Connected` → TCP stays open, cursor stops crossing, status dot turns orange.
- Resume → cursor crosses again without a new handshake.

---

## Phase 6b — `cursor-crossed` event (30 min)

**Problem.** Frontend Toast system wants to notify the user when the cursor has just handed off.

**Files**
- `src-tauri/src/coordinator.rs`

Inside the transition handler from `Local` → `Remote` (and vice versa), right after the state change commits:
```rust
let direction = match new_state {
    State::Remote => "to_remote",
    State::Local  => "to_local",
    _ => return,
};
let _ = self.app_handle.as_ref().map(|h| h.emit("cursor-crossed", serde_json::json!({
    "direction": direction,
    "peer_name": self.peer_name.clone().unwrap_or_default(),
})));
```

**Acceptance**
- Crossing the edge emits exactly one event per transition.
- Rapid back-and-forth emits one event per crossing (no dedupe necessary).

---

## Phase 6c — Persistent settings (2h)

**Problem.** `Advanced.svelte` has `launchOnLogin`, `autoReconnect`, `telemetry`, `theme`, `mdnsPort`, `fc_onboarded`. Currently only the last lives in `localStorage`. Others need to survive a Library reset and influence backend behavior.

**Files**
- `src-tauri/Cargo.toml` — add `tauri-plugin-store = "2"`
- `src-tauri/src/lib.rs` — register the plugin
- `src-tauri/src/settings.rs` (new) — typed loader/saver wrapping the store
- Wherever `autoReconnect` and `mdnsPort` are consumed

```rust
// settings.rs
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    pub launch_on_login: bool,
    pub auto_reconnect: bool,
    pub telemetry: bool,
    pub theme: String,        // "system" | "light" | "dark"
    pub mdns_port: u16,
}
impl Default for Settings { /* sane defaults, telemetry = false */ }

pub async fn load<R: Runtime>(app: &AppHandle<R>) -> Settings { /* ... */ }
pub async fn save<R: Runtime>(app: &AppHandle<R>, s: &Settings) -> Result<(), String> { /* ... */ }
```

Expose two commands: `get_settings()` and `set_settings(new: Settings)`. On `set_settings`, if `mdns_port` changed, restart the mDNS service; if `auto_reconnect` turned on and we have a known `last_peer_id`, kick off `start_client`.

Handle `launch_on_login` with `tauri-plugin-autostart` or the native macOS `LSUIElement` / Windows Startup folder approach — note in a TODO comment if you defer it.

**Acceptance**
- Toggling values persists across relaunches.
- Changing `mdns_port` live re-advertises on the new port.

---

## Phase 6d — Window config bump (5 min)

**File:** `src-tauri/tauri.conf.json`

```json
"windows": [{
  "title": "FlowControl",
  "width": 980,
  "height": 640,
  "minWidth": 840,
  "minHeight": 560,
  "resizable": true,
  "titleBarStyle": "Transparent",
  "hiddenTitle": true
}]
```

---

## Final checklist

Run in order after everything lands:
```bash
cd src-tauri && cargo fmt && cargo clippy -- -D warnings && cargo test
cd .. && bun run check && bun run tauri dev
```

Smoke test:
1. First launch → Welcome → Permissions (grant) → main window.
2. Start server on machine A, client on machine B → pair dialog appears on B with a 4-group code.
3. Accept → both sides show `Connected`, cursor crosses at the configured edge.
4. Pause from tray → cursor stops, banner appears. Resume → cursor crosses again.
5. Quit from tray → coordinator shuts down cleanly; relaunch preserves settings.

---

## Open questions for the maintainer

1. **Fingerprint strength.** The current design derives the fingerprint from unauthenticated TCP nonces — fine for "guard against typos on the wrong LAN" but not a real MITM defense. Upgrade to X25519 exchange before 1.0?
2. **OS detection.** The prototype shows a mac/win badge per peer. Include an `os` field in the mDNS TXT record or in `PairRequest.device_name`?
3. **Global shortcuts.** `Shortcuts.svelte` lists 5 hotkeys but none are registered yet. Use `tauri-plugin-global-shortcut` — acceptable to require accessibility permission for hotkeys too?
4. **Multi-peer.** `PeerRadar` supports N peers. Is the state machine ready for >2 participants, or should the UI enforce a hard 2-device cap?
5. **Telemetry.** Leave as a UI-only toggle with no backend, or wire a real opt-in pipeline (Sentry, etc)?

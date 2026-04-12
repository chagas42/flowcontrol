# FlowControl — Claude context

## What this project is

Software KVM switch that shares one physical mouse between a macOS machine and a Windows machine placed side by side. The user moves the cursor to the edge of one screen and it seamlessly transitions to the other. Inspired by Apple Universal Control, targeting macOS + Windows only.

## Stack

| Layer | Technology |
|---|---|
| Backend | Rust, Tauri v2 |
| Frontend | Svelte + TypeScript + Vite |
| Input (macOS) | CGEventTap (capture), CGEventPost (inject) |
| Input (Windows) | SetWindowsHookEx (capture), SendInput (inject) |
| Discovery | mDNS / Bonjour (`mdns-sd` crate) |
| Transport | TCP (reliability over marginal latency gains on LAN) |
| Wire format | `bincode` (compact; both endpoints are always same-version binary) |

## Module layout

```
src-tauri/src/
├── engine/           Pure business logic — zero OS deps, no async
│   ├── SPEC.md
│   ├── mod.rs
│   ├── screen_layout.rs    Coordinate types + ScreenLayout trait + ScreenLayoutImpl
│   ├── edge_detection.rs   EdgeDetection trait + EdgeDetectionImpl
│   ├── protocol.rs         Message enum (wire format types)
│   └── state_machine.rs    StateMachine trait + StateMachineImpl
├── input/            Platform input capture and injection
│   ├── SPEC.md
│   ├── mod.rs              InputCapture + InputInjector traits, InputEvent, errors
│   ├── macos.rs            TODO
│   └── windows.rs          TODO
├── network/          TCP connection + mDNS peer discovery
│   ├── SPEC.md
│   └── mod.rs              NetworkLayer trait, ConnectionState, Peer, errors — TODO impl
├── commands.rs       Tauri IPC bridge to frontend — TODO
├── lib.rs
└── main.rs
```

## Current state (as of commit fcceb10)

**Done:**
- All traits defined across all modules
- `engine` concrete implementations complete:
  - `ScreenLayoutImpl` — normalizes OS coords ↔ wire coords, derives watched edge from configured side
  - `EdgeDetectionImpl` — fires only on leading-edge of threshold zone, resets when cursor retreats
  - `StateMachineImpl` — full state transition table; `new()` seeds `Local`, `new_as_client()` seeds `Remote`
- SPEC.md co-located in each module directory
- `cargo check` passes (only "never used" warnings — expected; coordinator not yet wired)

**TODO (in implementation order):**
1. `network/` implementation — TCP server/client + mDNS peer discovery
2. `input/macos.rs` — CGEventTap capture + CGEventPost injection
3. `input/windows.rs` — SetWindowsHookEx + SendInput
4. `commands.rs` — Tauri IPC bridge
5. Coordinator — wires all modules together, drives state machine, executes commands
6. Frontend — `App.svelte`, `ArrangeDisplays.svelte` (drag-drop layout), `ConnectionStatus.svelte`, permissions UI
7. CI/CD — GitHub Actions for macOS + Windows builds, `.dmg` + `.msi` installers

## Key design decisions

**State machine is symmetric.** Both machines run `StateMachineImpl`. The coordinator seeds initial state:
- Server (physical mouse): `StateMachineImpl::new()` → starts `Local`
- Client (receives cursor): `StateMachineImpl::new_as_client()` → starts `Remote`

**Coordinate types are intentionally distinct.** `Point { x: f64, y: f64 }` for OS coordinates, `NormalizedPoint { x: f32, y: f32 }` for wire coordinates (always `[0.0, 1.0]`). Only `ScreenLayoutImpl` converts between them.

**Engine has no OS dependencies.** Every type in `engine/` compiles identically on macOS, Windows, Linux. No `#[cfg]` inside engine.

**`#[cfg(target_os)]` at compile time** for input implementations — no runtime dispatch on the hot path (up to 240Hz mouse events).

**Bounded channel for backpressure.** `InputCapture::start` takes `tokio::sync::mpsc::Sender<InputEvent>` (capacity 64). Implementation must use `try_send` — blocking the CGEventTap callback causes macOS to disable the tap.

## Workflow rules

- Enter plan mode before every implementation. Never write code without an approved plan.
- Each new module must have a co-located `SPEC.md` created as part of implementation.
- `cargo check` must pass with zero errors after every session.

## Commit convention

Conventional commits, short imperative subject. No Claude co-authorship.
```
feat(module): description
fix(module): description
refactor: description
```

## Verify

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

---

## Last session resume

> This section is rewritten at the end of every session so any machine can pick up from the exact same state.

**Date:** 2026-04-11
**Commit:** pending — `coordinator.rs` written, `cargo check` passes (89 "never used" warnings, zero errors)

### What was done this session

**`coordinator.rs` — `src/coordinator.rs` (new file):**
- `Coordinator` struct owns `StateMachineImpl`, `ScreenLayoutImpl`, `EdgeDetectionImpl`, `NetworkLayerImpl`, `MacOSCapture`, `MacOSInjector` (platform-gated)
- Two constructors: `new_server(local_dims, side)` → `StateMachineImpl::new()` (Local start); `new_client(local_dims, side)` → `StateMachineImpl::new_as_client()` (Remote start)
- `run_as_server(name)` / `run_as_client()` — start network, start capture, enter `event_loop()`
- `event_loop()` — `tokio::select!` on `input_rx` and `network_rx`
- `on_input`: Local/Transitioning → edge detection → state machine; Remote → forward over network
- `on_network`: Connected → `ConnectionEstablished` + send `ScreenInfo`; Disconnected → `ConnectionLost`; MessageReceived → `on_message()`
- `on_message`: `MouseMove/Button/Scroll` → inject; `TransitionIn/Ack` → state machine; `ScreenInfo` → configure `ScreenLayout` + `EdgeDetection`
- `execute_commands`: `StartForwarding` → `suppressing=true` + hide cursor; `StopForwarding` → `suppressing=false`; `AcceptCursor` → show cursor + inject at entry point; `Send` → `network.send()`
- `lib.rs` updated with `mod coordinator;`

### Next tasks (in order)

1. **`input::windows`** — `SetWindowsHookEx` capture + `SendInput` injection (enter plan mode first)
   - `WindowsCapture` + `WindowsInjector`, all `#[cfg(target_os = "windows")]`
   - No permission needed — `permission_status()` always `Granted`
   - `stop()` → `UnhookWindowsHookEx` + `PostThreadMessage(WM_QUIT)`

2. **`commands.rs`** — Tauri IPC bridge
   - Expose `start_server`, `start_client`, `connect_to_peer`, `get_peers`, `stop` as `#[tauri::command]`
   - Store `Arc<Mutex<Option<Coordinator>>>` in Tauri state; run coordinator in `tokio::spawn`

3. **Frontend** — `App.svelte`, `ArrangeDisplays.svelte`, `ConnectionStatus.svelte`, permissions UI

4. **CI/CD** — GitHub Actions, `.dmg` + `.msi` installers

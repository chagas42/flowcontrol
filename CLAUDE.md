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

**Date:** 2026-04-12
**Commit:** `1a24a7a` — feat(input): implement MacOSCapture and MacOSInjector via CGEventTap

### What was done this session

**Contract fixes (before input::macos):**
- `inject_move` now takes `Point` (pixel coords) instead of `(x_norm, y_norm)` — coordinator owns coordinate conversion via `ScreenLayout`, single source of truth
- Added `Message::Ack` to protocol — `TransitionInReceived` now emits `Send(Ack)` so the other machine can fire `TransitionAcknowledged → Remote + StartForwarding`
- Confirmed all module contracts are clean: `engine` has zero OS/network deps; `input` and `network` depend only on engine data types

**`input::macos` — `src/input/macos.rs`:**
- `MacOSCapture` — CGEventTap via raw FFI, `extern "C"` callback, `std::thread::spawn` for `CFRunLoopRun`
- `MacOSInjector` — `CGWarpMouseCursorPosition` + `CGEventPost` for move, button, scroll; `CGDisplayHideCursor/ShowCursor` + `CGAssociateMouseAndMouseCursorPosition` for cursor visibility
- `suppressing: Arc<AtomicBool>` — coordinator sets this on `StartForwarding`; callback returns `null` to suppress local events while cursor is on remote machine
- `permission_status()` → `AXIsProcessTrusted()`; `request_permission()` → `AXIsProcessTrustedWithOptions` with `kAXTrustedCheckOptionPrompt = true`
- Raw pointer → thread boundary: cast to `usize` before `spawn`, cast back inside thread
- `cargo check` passes — 87 "never used" warnings only (expected)

### Next task

Implement `input::windows` — SetWindowsHookEx capture + SendInput injection. Enter plan mode first.

Scope:
- `WindowsCapture` struct implementing `InputCapture` — `SetWindowsHookEx(WH_MOUSE_LL, ...)`, message loop on dedicated thread
- `WindowsInjector` struct implementing `InputInjector` — `SendInput` for move, button, scroll; `ShowCursor(FALSE/TRUE)` for visibility
- No permission check needed on Windows — `permission_status()` always returns `Granted`
- `stop()` → `UnhookWindowsHookEx` + `PostThreadMessage(WM_QUIT)` to exit message loop
- All code gated with `#[cfg(target_os = "windows")]`

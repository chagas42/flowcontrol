# network — Design Spec

## Purpose

Manages the TCP connection between two machines and handles automatic peer discovery via mDNS. Exposes a single `NetworkLayer` trait so the rest of the codebase is isolated from all networking details.

## TCP over UDP

UDP would offer lower theoretical latency but requires implementing reliability, ordering, and congestion control manually. On a local network the RTT is under 1ms and TCP's overhead is negligible. The reliability guarantee matters: a dropped `TransitionIn` message would leave the state machine in an inconsistent state with no recovery path. TCP was chosen for correctness over marginal latency gains.

## mDNS for discovery

Both machines advertise a `_flowcontrol._tcp` Bonjour service. This means:

- No manual IP configuration required.
- Works on any standard LAN including home routers.
- macOS has native Bonjour support. Windows supports mDNS via `mdns-sd` (pure Rust, no native dependency).

Fallback to manual IP entry will be added in a later iteration.

## Wire format

Messages are serialized with `bincode` (compact binary, not JSON). Reasons:

- `MouseMove` fires up to 240 times per second. JSON adds ~30 bytes of overhead per message vs ~9 bytes for bincode.
- Both endpoints are always FlowControl binaries (same version), so human-readable format provides no benefit.
- `bincode` is already a dependency via `serde`.

## ConnectionState role in handshake

The `ConnectionState` enum drives the UI status indicator and gates the state machine. `ScreenInfo` messages are exchanged immediately after `Connected` — only once both machines have each other's screen dimensions can the `ScreenLayout` be configured and edge detection begin. The coordinator enforces this ordering.

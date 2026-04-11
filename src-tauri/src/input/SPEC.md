# input — Design Spec

## Purpose

Platform-specific input capture and injection. Abstracts the OS input APIs behind two traits so the rest of the codebase never references any platform API directly.

## Why two separate traits

`InputCapture` and `InputInjector` are intentionally split:

- On the **server** machine (has the physical mouse): only `InputCapture` is active.
- On the **client** machine (receives events): only `InputInjector` is active.
- A machine is never both at the same time (single-mouse assumption).

Merging them into one trait would force every implementation to stub out half its methods.

## Platform permission model

| Platform | Requirement | Prompt needed |
|----------|-------------|---------------|
| macOS | Accessibility permission (`AXIsProcessTrusted`) | Yes — System Settings |
| Windows | None | No |

This asymmetry is why `permission_status()` and `request_permission()` exist on the trait — Windows implementations return `Granted` immediately and no-op on request.

## Bounded channel for backpressure

`InputCapture::start` takes `tokio::sync::mpsc::Sender<InputEvent>` (bounded). The CGEventTap callback on macOS fires at up to 240Hz on ProMotion displays. If the consumer falls behind, the implementation must use `try_send` and drop the event rather than blocking the OS callback thread. Blocking the callback thread causes the system to disable the event tap.

Recommended channel capacity: 64 events (~266ms of buffer at 240Hz).

## `#[cfg]` gates over a plugin system

Platform implementations are selected at compile time via `#[cfg(target_os)]`, not at runtime. This avoids dynamic dispatch overhead on the hot path (every mouse event) and produces a smaller binary with no dead code for the target platform. A runtime plugin system would add complexity with no benefit since each binary targets exactly one OS.

# engine — Design Spec

## Purpose

Pure business logic with zero OS dependencies. Every type and trait in this module compiles identically on macOS, Windows, and Linux. No system calls, no I/O, no async runtime — only data structures and trait definitions that describe what the system does, not how it does it.

## Module dependency order

```
screen_layout   (no deps within engine)
     ↓
edge_detection  (depends on screen_layout types)
     ↓
protocol        (depends on screen_layout::ScreenDimensions)
     ↓
state_machine   (depends on edge_detection + protocol)
```

## Coordinate system decisions

Two coordinate types are intentional and distinct:

- `Point { x: f64, y: f64 }` — OS screen coordinates. f64 matches what CGEventTap and Win32 give us. Used only inside this process.
- `NormalizedPoint { x: f32, y: f32 }` — Wire coordinates, always in [0.0, 1.0]. f32 is sufficient precision for screen position and halves the size on the wire. Used in `Message` and `EdgeCrossedEvent`.

Converting between them is the responsibility of `ScreenLayout`. Nothing else in the codebase should do coordinate math.

## Trait-only design

No concrete implementations live in `engine`. Every module exposes only traits and data types. This means:

- Implementations can be swapped without touching any shared logic.
- Unit tests for `state_machine` or `screen_layout` logic need no OS mocks.
- The iMac (VSCode only, no Xcode) can contribute to this module without needing to compile platform-specific code.

## State machine command pattern

`StateMachine::handle` returns `Vec<Command>` rather than calling callbacks directly. This keeps the state machine free of dependencies on the network or input layers, making it fully testable in isolation. The coordinator (not yet implemented) is responsible for executing the returned commands.

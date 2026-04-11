# FlowControl

Share a single mouse between macOS and Windows machines placed side by side. When the cursor reaches the configured screen edge, it transitions seamlessly to the other machine over the local network.

Inspired by Apple's Universal Control — built open source for cross-platform use.

## Features

- Works between macOS and Windows
- Automatic peer discovery via mDNS — no IP configuration needed
- Visual display arrangement: drag displays to match your physical setup
- Lives in the system tray, no dock icon
- Low latency — direct TCP connection on the local network

## Status

Early development. Not functional yet.

| Module | Status |
|--------|--------|
| Core (protocol, state machine, layout) | Defined |
| Network layer | Planned |
| Input capture — macOS | Planned |
| Input capture — Windows | Planned |
| UI (Tauri + Svelte) | Scaffolded |

## Architecture

Rust backend (Tauri) handles all system-level work: input capture, injection, and networking. Svelte frontend provides the configuration UI. The two communicate through typed Tauri commands.

```
src-tauri/src/
├── core/          # protocol, state machine, screen layout, edge detection
├── input/         # platform-specific input capture and injection
│   ├── macos.rs   # CGEventTap + CGEventPost
│   └── windows.rs # SetWindowsHookEx + SendInput
├── network/       # TCP + mDNS peer discovery
└── commands.rs    # Tauri bridge to frontend

src/               # Svelte frontend
└── lib/
    ├── ArrangeDisplays.svelte
    └── ConnectionStatus.svelte
```

## Requirements

- macOS 13+ or Windows 10+
- Both machines on the same local network
- macOS: Accessibility permission required for input capture

## Building

```bash
git clone https://github.com/chagas42/flowcontrol.git
cd flowcontrol
npm install
cargo tauri dev
```

## Contributing

Open an issue before submitting a pull request. The project is in early development and the architecture may still change.

## License

MIT

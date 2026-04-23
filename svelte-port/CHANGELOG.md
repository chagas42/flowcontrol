# FlowControl — Port Manifest (for Claude Code)

**Read me first.** This folder contains the full frontend port of FlowControl's UI. The target repo uses Svelte 5 + Tauri v2. Every file here is a drop-in replacement or addition — the paths below map source → destination.

## Step 1: Copy frontend files

```bash
# From your repo root:
mkdir -p src/lib/{pair,radar,toast,pause,onboarding,screens}

# Root
cp svelte-port/app.css                       src/app.css
cp svelte-port/App.svelte                    src/App.svelte

# Shared components
cp svelte-port/ArrangeDisplays.svelte        src/lib/ArrangeDisplays.svelte
cp svelte-port/Sidebar.svelte                src/lib/Sidebar.svelte
cp svelte-port/NavIcon.svelte                src/lib/NavIcon.svelte
cp svelte-port/StatusDot.svelte              src/lib/StatusDot.svelte
cp svelte-port/StatusStrip.svelte            src/lib/StatusStrip.svelte
cp svelte-port/Kbd.svelte                    src/lib/Kbd.svelte
cp svelte-port/PeerCard.svelte               src/lib/PeerCard.svelte
cp svelte-port/PeerCardSkeleton.svelte       src/lib/PeerCardSkeleton.svelte
cp svelte-port/EmptyState.svelte             src/lib/EmptyState.svelte
cp svelte-port/OSBadge.svelte                src/lib/OSBadge.svelte

# Feature clusters
cp svelte-port/pair/PairRequestDialog.svelte src/lib/pair/PairRequestDialog.svelte
cp svelte-port/radar/PeerRadar.svelte        src/lib/radar/PeerRadar.svelte
cp svelte-port/toast/toastStore.svelte       src/lib/toast/toastStore.svelte
cp svelte-port/toast/ToastLayer.svelte       src/lib/toast/ToastLayer.svelte
cp svelte-port/pause/PauseBanner.svelte      src/lib/pause/PauseBanner.svelte
cp svelte-port/onboarding/Welcome.svelte     src/lib/onboarding/Welcome.svelte
cp svelte-port/onboarding/Permissions.svelte src/lib/onboarding/Permissions.svelte
cp svelte-port/screens/Shortcuts.svelte      src/lib/screens/Shortcuts.svelte
cp svelte-port/screens/About.svelte          src/lib/screens/About.svelte
cp svelte-port/screens/Advanced.svelte       src/lib/screens/Advanced.svelte
```

## Step 2: Remove obsolete files

```bash
rm -f src/lib/Counter.svelte
rm -f src/lib/ConnectionStatus.svelte   # status now lives in Sidebar/StatusStrip
```

## Step 3: Tauri config

Patch `src-tauri/tauri.conf.json` — bump window and enable resize:
```json
"windows": [{
  "title": "FlowControl",
  "width": 980,
  "height": 640,
  "minWidth": 840,
  "minHeight": 560,
  "resizable": true
}]
```

## Step 4: Backend

See `BACKEND_IMPLEMENTATION.md` in the repo root for the full backend plan (phases 4a through 6d). The frontend already consumes these Tauri commands and events — implement them in order:

**Commands invoked** (must exist before frontend works end-to-end):
| Command | Phase | File delivered |
|---|---|---|
| `start_server`, `start_client`, `stop_coordinator` | (existing) | — |
| `check_accessibility_permission`, `request_accessibility_permission` | (existing) | — |
| `connect_to_peer` | (existing) | — |
| `pair_accept`, `pair_decline` | 4b | `PairRequestDialog.svelte` |
| `pause_sharing`, `resume_sharing` | 6a | `PauseBanner.svelte`, tray |

**Events listened to:**
| Event | Payload | Phase | Consumer |
|---|---|---|---|
| `peers-updated` | `[{id, name, os?}]` | (existing) | `App.svelte`, `PeerRadar` |
| `status-changed` | `"Connected" \| "Remote" \| "Searching" \| "Disconnected" \| "Paused" \| "Stopped"` | 4a | `App.svelte`, `Sidebar`, `StatusStrip` |
| `permission-required` | — | (existing) | `App.svelte` |
| `pair-incoming` | `{peer_name, fingerprint, os}` | 4b | `App.svelte` → `PairRequestDialog` |
| `pair-resolved` | — | 4b | `App.svelte` |
| `cursor-crossed` | `{direction, peer_name}` | 6b | `App.svelte` → toast |
| `tray-action` | `"pause" \| "about"` | 5 | `App.svelte` |

## Step 5: Verify

```bash
bun install
bun run check     # svelte-check — should pass with 0 errors
bun run tauri dev
```

**Smoke test:**
1. Fresh launch → Welcome screen → Get started → Permissions → Continue → main window.
2. Start server. Status says "Searching". Radar appears while peers is empty.
3. On second device, pair request modal shows 4-group fingerprint. Accept → both devices show `Connected`.
4. Cross screen edge → toast "Cursor crossed to …".
5. Tray menu → Pause → banner appears, status turns "Paused". Resume → back to Connected.
6. Sidebar → Shortcuts / About / Advanced → all render, Advanced theme toggle switches light/dark instantly.
7. Quit app, reopen → lands directly in main (onboarding skipped).

## File map cheatsheet

```
svelte-port/
├── App.svelte                 → root orchestrator (routing, event listeners)
├── app.css                    → design tokens + dark mode
├── Sidebar.svelte             → left nav (two-way bind on `active`)
├── StatusStrip.svelte         → header banner (slot for pair actions)
├── ArrangeDisplays.svelte     → 4-edge snap canvas (unchanged from phase 1)
├── PeerCard / Skeleton        → device cards
├── EmptyState                 → first-run "no peers yet"
├── OSBadge                    → mac/win icon
├── Kbd / StatusDot / NavIcon  → atoms
│
├── onboarding/
│   ├── Welcome.svelte         → hero + CTA
│   └── Permissions.svelte     → accessibility polling
│
├── pair/
│   └── PairRequestDialog.svelte  → modal w/ 4-group fingerprint
│
├── radar/
│   └── PeerRadar.svelte       → concentric rings + peer markers
│
├── toast/
│   ├── toastStore.svelte      → writable store + pushToast()
│   └── ToastLayer.svelte      → mount once at root
│
├── pause/
│   └── PauseBanner.svelte     → shows when status === 'Paused'
│
└── screens/
    ├── Shortcuts.svelte       → global hotkey list
    ├── About.svelte           → version, license, links
    └── Advanced.svelte        → toggles + theme + mDNS port
```

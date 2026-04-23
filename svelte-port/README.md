# Port instructions

Arquivos em `svelte-port/` são drop-in pro seu repo. Copia assim:

```
svelte-port/app.css                       → src/app.css
svelte-port/App.svelte                    → src/App.svelte
svelte-port/ArrangeDisplays.svelte        → src/lib/ArrangeDisplays.svelte
svelte-port/Sidebar.svelte                → src/lib/Sidebar.svelte
svelte-port/NavIcon.svelte                → src/lib/NavIcon.svelte
svelte-port/StatusDot.svelte              → src/lib/StatusDot.svelte
svelte-port/StatusStrip.svelte            → src/lib/StatusStrip.svelte
svelte-port/Kbd.svelte                    → src/lib/Kbd.svelte
svelte-port/PeerCard.svelte               → src/lib/PeerCard.svelte
svelte-port/OSBadge.svelte                → src/lib/OSBadge.svelte
svelte-port/onboarding/Welcome.svelte     → src/lib/onboarding/Welcome.svelte
svelte-port/onboarding/Permissions.svelte → src/lib/onboarding/Permissions.svelte
```

## Backend (Fase 4a — status granularity)

No `coordinator.rs`, após cada transição ou evento de rede, emita `status-changed` com a string alinhada ao enum do frontend:

```rust
// após state machine ou network mudar
let status = match (self.sm.state(), self.net.state()) {
    (State::Local, ConnectionState::Connected) => "Connected",
    (State::Remote, _)                         => "Remote",
    (_, ConnectionState::Browsing)             => "Searching",
    (_, ConnectionState::Disconnected)         => "Disconnected",
    _                                          => "Searching",
};
if let Some(h) = &self.app_handle { let _ = h.emit("status-changed", status); }
```

## Ajustes em `tauri.conf.json`

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

## Remover

- `src/lib/Counter.svelte` (não usado).
- `ConnectionStatus.svelte` continua válido, mas agora o status vive no `Sidebar`. Pode remover se não usar em mais nada.

## Próximos passos

- **Fase 4b** — pair request + fingerprint (maior adição de backend).
- **Fase 5** — tray menu dinâmico.
- **Fase 6** — toast "cursor crossed" + dark mode já funciona via `prefers-color-scheme`.

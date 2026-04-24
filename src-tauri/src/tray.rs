//! System tray icon + dynamic menu.
//!
//! One-shot build in `lib::run.setup()` via [`build_tray`]. The coordinator
//! calls [`rebuild_menu`] whenever peers or status change so the quick-actions
//! menu stays in sync. `Open`, `Pause`, and `About` emit `tray-action` events
//! that the frontend listens for; `Quit` tears the coordinator down and exits.

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

/// Registered id of our tray icon. Used by `get_tray_by_id` when rebuilding
/// the menu after peers/status change.
const TRAY_ID: &str = "fc-tray";

/// Build the tray icon the first time the app boots.
pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = build_menu(app, &[], "Stopped")?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().unwrap().clone())
        .icon_as_template(true) // monochrome on macOS menu bar
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "pause" => {
                let _ = app.emit("tray-action", "pause");
            }
            "about" => {
                let _ = app.emit("tray-action", "about");
            }
            "quit" => {
                // Best-effort: notify the frontend so it stops the coordinator
                // (start_server/client are async and we don't own a handle to
                // AppState here cleanly). If the user wants a hard stop now,
                // app.exit is the belt-and-suspenders.
                let _ = app.emit("tray-action", "quit");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// Rebuild the menu with the current peer list + status. The coordinator calls
/// this whenever `peers-updated` or `status-changed` fires so the quick-actions
/// stay live.
pub fn rebuild_menu<R: Runtime>(
    app: &AppHandle<R>,
    peers: &[PeerView],
    status: &str,
) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    let menu = build_menu(app, peers, status)?;
    tray.set_menu(Some(menu))?;
    Ok(())
}

/// Lightweight peer view we need for the menu. Stays independent of the
/// `network::Peer` type so this module doesn't pull in network internals.
pub struct PeerView {
    pub name: String,
    pub online: bool,
}

fn build_menu<R: Runtime>(
    app: &AppHandle<R>,
    peers: &[PeerView],
    status: &str,
) -> tauri::Result<tauri::menu::Menu<R>> {
    let status_item = MenuItemBuilder::with_id("status", format!("Status: {status}"))
        .enabled(false)
        .build(app)?;
    let open = MenuItemBuilder::with_id("open", "Open FlowControl").build(app)?;
    let pause_label = if status == "Paused" {
        "Resume sharing"
    } else {
        "Pause sharing"
    };
    let pause = MenuItemBuilder::with_id("pause", pause_label).build(app)?;

    let mut builder = MenuBuilder::new(app).items(&[
        &status_item,
        &PredefinedMenuItem::separator(app)?,
        &open,
        &pause,
        &PredefinedMenuItem::separator(app)?,
    ]);

    if peers.is_empty() {
        let empty = MenuItemBuilder::with_id("peers-empty", "No peers discovered")
            .enabled(false)
            .build(app)?;
        builder = builder.item(&empty);
    } else {
        for (i, peer) in peers.iter().enumerate() {
            let dot = if peer.online { "🟢" } else { "🔴" };
            let label = format!("{dot} {}", peer.name);
            // IDs are informational only — we don't react to peer clicks yet.
            let id = format!("peer-{i}");
            let item = MenuItemBuilder::with_id(id, label)
                .enabled(false)
                .build(app)?;
            builder = builder.item(&item);
        }
    }

    let about = MenuItemBuilder::with_id("about", "About FlowControl").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    builder = builder.items(&[&PredefinedMenuItem::separator(app)?, &about, &quit]);

    builder.build()
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

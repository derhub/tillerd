//! Native menu wiring. The platform-default menu gains a `View` submenu with a Logs entry (routes
//! the renderer to `/logs`) and a Command Center entry whose accelerator is the rebindable leader
//! key — pressed, it emits `command-center:open`, which the renderer opens the palette on. The
//! accelerator fires regardless of webview focus, so it reaches the command center even while a
//! terminal holds keyboard focus. The leader item handle is held in managed state so the renderer
//! can rebind it through `command_center_set_leader`.

use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem, MenuItemBuilder, MenuItemKind, SubmenuBuilder};
use tauri::{Emitter, Manager, State, Wry};

/// Default leader accelerator — the common palette convention; rebindable from settings.
pub const DEFAULT_LEADER_ACCEL: &str = "CmdOrCtrl+K";

/// Where a menu id routes when its item fires: a renderer event and its payload.
pub struct MenuRoute {
    pub event: &'static str,
    pub payload: &'static str,
}

/// Pure mapping from a menu item id to the event it emits. Kept separate from the menu build so the
/// id -> event contract is unit-testable without a running app.
pub fn menu_event_route(id: &str) -> Option<MenuRoute> {
    match id {
        "view_logs" => Some(MenuRoute {
            event: "menu:navigate",
            payload: "/logs",
        }),
        "command_center" => Some(MenuRoute {
            event: "command-center:open",
            payload: "",
        }),
        _ => None,
    }
}

/// Holds the leader menu item so its accelerator can be rebound at runtime.
#[derive(Default)]
pub struct LeaderMenuState(pub Mutex<Option<MenuItem<Wry>>>);

/// Build the menu, wire menu events to renderer emits, and stash the leader item for rebinding.
pub fn install_menu(app: &tauri::App) -> tauri::Result<()> {
    let logs = MenuItemBuilder::with_id("view_logs", "Logs").build(app)?;
    let leader = MenuItemBuilder::with_id("command_center", "Command Center")
        .accelerator(DEFAULT_LEADER_ACCEL)
        .build(app)?;

    let menu = Menu::default(app.handle())?;
    let mut placed = false;
    for item in menu.items()? {
        if let MenuItemKind::Submenu(sub) = item {
            if sub.text().unwrap_or_default() == "View" {
                sub.append(&logs)?;
                sub.append(&leader)?;
                placed = true;
                break;
            }
        }
    }
    if !placed {
        menu.append(
            &SubmenuBuilder::new(app, "View")
                .item(&logs)
                .item(&leader)
                .build()?,
        )?;
    }
    app.set_menu(menu)?;
    app.on_menu_event(|app_handle, event| {
        if let Some(route) = menu_event_route(event.id().as_ref()) {
            let _ = app_handle.emit(route.event, route.payload);
        }
    });

    *app.state::<LeaderMenuState>().0.lock().unwrap() = Some(leader);
    Ok(())
}

/// Rebind the native leader accelerator. No-op until the leader item is installed.
#[tauri::command]
pub fn command_center_set_leader(
    accelerator: String,
    state: State<LeaderMenuState>,
) -> Result<(), String> {
    if let Some(item) = state.0.lock().unwrap().as_ref() {
        item.set_accelerator(Some(accelerator))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_center_id_emits_the_open_event() {
        let route = menu_event_route("command_center").expect("command_center is routed");
        assert_eq!(route.event, "command-center:open");
    }

    #[test]
    fn logs_id_navigates_to_logs() {
        let route = menu_event_route("view_logs").expect("view_logs is routed");
        assert_eq!(route.event, "menu:navigate");
        assert_eq!(route.payload, "/logs");
    }

    #[test]
    fn unknown_id_has_no_route() {
        assert!(menu_event_route("nope").is_none());
    }

    #[test]
    fn default_leader_is_a_chord_accelerator() {
        assert!(DEFAULT_LEADER_ACCEL.contains('+'));
    }
}

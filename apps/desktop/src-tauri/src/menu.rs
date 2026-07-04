//! Native menu wiring. The platform-default menu gains a `View` submenu with a Logs entry (routes
//! the renderer to `/logs`) and a Command Center entry whose accelerator is the rebindable leader
//! key -- pressed, it emits `command-center:open`, which the renderer opens the palette on. The
//! accelerator fires regardless of webview focus, so it reaches the command center even while a
//! terminal holds keyboard focus. The leader item handle is held in managed state so the renderer
//! can rebind it through `command_center_set_leader`.
//!
//! The `File` submenu gains New Project / New Session / New Terminal / Close Panel / Search
//! Sessions entries. Their ids are the palette's own command ids (see `ACTION` in
//! `apps/ui/app/lib/commands/ids.ts`); each routes to a generic `menu:command` event carrying that
//! id as payload, so the renderer dispatches through the same command registry the palette uses --
//! a menu item and its palette entry can never disagree on behavior.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::menu::{
    Menu, MenuItem, MenuItemBuilder, MenuItemKind, PredefinedMenuItem, SubmenuBuilder,
};
use tauri::{Emitter, Manager, State, Wry};

/// Default leader accelerator -- the common palette convention; rebindable from settings.
pub const DEFAULT_LEADER_ACCEL: &str = "CmdOrCtrl+K";

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "menu:navigate")]
pub struct MenuNavigate(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "command-center:open")]
pub struct CommandCenterOpen(pub String);

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
        // Palette command ids, forwarded verbatim as the event payload so the renderer can
        // dispatch through the command registry by id.
        "project.new" => Some(MenuRoute {
            event: "menu:command",
            payload: "project.new",
        }),
        "session.new" => Some(MenuRoute {
            event: "menu:command",
            payload: "session.new",
        }),
        "surface.spawn" => Some(MenuRoute {
            event: "menu:command",
            payload: "surface.spawn",
        }),
        "surface.close" => Some(MenuRoute {
            event: "menu:command",
            payload: "surface.close",
        }),
        "session.search" => Some(MenuRoute {
            event: "menu:command",
            payload: "session.search",
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

    // Accelerators mirror the palette's own defaults for these ids (see
    // apps/ui/app/lib/commands/defs.ts) so the native menu and the palette never show
    // conflicting shortcuts for the same command.
    let project_new = MenuItemBuilder::with_id("project.new", "New Project")
        .accelerator("CmdOrCtrl+Shift+N")
        .build(app)?;
    let session_new = MenuItemBuilder::with_id("session.new", "New Session")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let surface_spawn = MenuItemBuilder::with_id("surface.spawn", "New Terminal")
        .accelerator("CmdOrCtrl+T")
        .build(app)?;
    let surface_close = MenuItemBuilder::with_id("surface.close", "Close Panel")
        .accelerator("CmdOrCtrl+W")
        .build(app)?;
    let session_search = MenuItemBuilder::with_id("session.search", "Search Sessions")
        .accelerator("CmdOrCtrl+P")
        .build(app)?;

    let menu = Menu::default(app.handle())?;
    let mut view_placed = false;
    let mut file_placed = false;
    for item in menu.items()? {
        let Some(sub) = (match item {
            MenuItemKind::Submenu(sub) => Some(sub),
            _ => None,
        }) else {
            continue;
        };
        match sub.text().unwrap_or_default().as_str() {
            "View" => {
                sub.append(&logs)?;
                sub.append(&leader)?;
                view_placed = true;
            }
            // `Menu::default` only builds a `File` submenu on macOS/Windows; prepend so these
            // entries lead the platform defaults (Close Window, Quit) rather than trailing them.
            "File" => {
                sub.prepend(&session_search)?;
                sub.prepend(&PredefinedMenuItem::separator(app)?)?;
                sub.prepend(&surface_close)?;
                sub.prepend(&surface_spawn)?;
                sub.prepend(&session_new)?;
                sub.prepend(&project_new)?;
                file_placed = true;
            }
            _ => {}
        }
        if view_placed && file_placed {
            break;
        }
    }
    if !view_placed {
        menu.append(
            &SubmenuBuilder::new(app, "View")
                .item(&logs)
                .item(&leader)
                .build()?,
        )?;
    }
    if !file_placed {
        // Linux gets no default `File` submenu at all -- build one from scratch.
        menu.append(
            &SubmenuBuilder::new(app, "File")
                .item(&project_new)
                .item(&session_new)
                .item(&surface_spawn)
                .item(&surface_close)
                .separator()
                .item(&session_search)
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
#[specta::specta]
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

    #[test]
    fn palette_command_ids_route_to_menu_command_with_their_own_id_as_payload() {
        for id in [
            "project.new",
            "session.new",
            "surface.spawn",
            "surface.close",
            "session.search",
        ] {
            let route = menu_event_route(id).unwrap_or_else(|| panic!("{id} is routed"));
            assert_eq!(route.event, "menu:command");
            assert_eq!(route.payload, id);
        }
    }
}

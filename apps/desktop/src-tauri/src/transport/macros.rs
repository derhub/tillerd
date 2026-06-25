//! Declarative shim macros for the tauri transport. Each domain operation is a pure
//! `app/` CQS command/query value; these macros generate the mechanical
//! `#[tauri::command]` shim that deserializes the wire arguments, dispatches through the
//! managed `Bus<Ctx>`, and maps `shared::Error` to the wire string -- so adding an op is
//! one listing line. The wire (command name, argument shape, response JSON, error
//! string) is byte-identical to a hand-written shim: the generated fn takes the same
//! snake_case params (tauri converts the renderer's camelCase natively) and tauri keeps
//! routing, argument typing, and per-command ACL.
//!
//! Three forms cover the mechanical surface:
//! - [`transport_command!`] -- build a command, `bus.execute`, return `()`.
//! - [`transport_query!`] -- `bus.query`, map the output to the curated wire DTO.
//! - [`transport_create!`] -- the pure-CQS create pattern: build the command (with a
//!   transport-minted id), `bus.execute`, then `bus.query` the entity back by id and map
//!   it to the wire DTO. Minting lives here, in one place, because the core command
//!   returns `()` (CQS stays pure).
//!
//! [`collect_transport!`] expands to the `generate_handler![...]` array -- declarative,
//! not `inventory`, because tauri needs the handler idents at compile time. Host/shell
//! and transport-resident shims (window/log/bridge/menu/supervisor/gate/store, the
//! surface I/O channel, and the few off-bus or non-mechanical domain shims) are NOT
//! macro-generated and are listed alongside in `collect_transport!`.

/// Generate a `#[tauri::command]` shim for a command (mutation returning `()`).
///
/// `$param: $ty = $build` declares one wire argument and how it maps into the core
/// field: `$build` is an expression in scope of the param binding (e.g. an id-newtype
/// constructor). The `=> Core { field: expr, .. }` clause is the core command value the
/// bus executes.
macro_rules! transport_command {
    (
        $(#[$meta:meta])*
        $name:ident ( $( $param:ident : $ty:ty ),* $(,)? ) => $build:expr
    ) => {
        $(#[$meta])*
        #[tauri::command]
        #[specta::specta]
        #[allow(clippy::too_many_arguments)] // generated transport shim; arg count mirrors the wire command
        pub async fn $name(
            $( $param: $ty, )*
            bus: tauri::State<'_, $crate::transport::Bus>,
        ) -> ::std::result::Result<(), ::std::string::String> {
            bus.execute($build).await.map_err(|e| e.to_string())
        }
    };
}

/// Generate a `#[tauri::command]` shim for a query. `=> $query` is the core query value;
/// `|$out| $map` maps its output to the wire response (Vec/Option/DTO/passthrough).
macro_rules! transport_query {
    (
        $(#[$meta:meta])*
        $name:ident ( $( $param:ident : $ty:ty ),* $(,)? ) -> $ret:ty
            => $query:expr , | $out:ident | $map:expr
    ) => {
        $(#[$meta])*
        #[tauri::command]
        #[specta::specta]
        #[allow(clippy::too_many_arguments)] // generated transport shim; arg count mirrors the wire command
        pub async fn $name(
            $( $param: $ty, )*
            bus: tauri::State<'_, $crate::transport::Bus>,
        ) -> ::std::result::Result<$ret, ::std::string::String> {
            let $out = bus.query($query).await.map_err(|e| e.to_string())?;
            ::std::result::Result::Ok($map)
        }
    };
}

/// Generate a `#[tauri::command]` shim for a create: build the command (typically with a
/// transport-minted id captured before `=>`), execute it, then query the entity back by
/// id and map it to the wire DTO. `let $bind = $mint;` runs first so the same id flows
/// into both the command and the read-back query.
macro_rules! transport_create {
    (
        $(#[$meta:meta])*
        $name:ident ( $( $param:ident : $ty:ty ),* $(,)? ) -> $ret:ty {
            let $bind:ident = $mint:expr;
            execute: $cmd:expr,
            read_back: $query:expr,
            map: | $out:ident | $map:expr,
            missing: $missing:expr $(,)?
        }
    ) => {
        $(#[$meta])*
        #[tauri::command]
        #[specta::specta]
        #[allow(clippy::too_many_arguments)] // generated transport shim; arg count mirrors the wire command
        pub async fn $name(
            $( $param: $ty, )*
            bus: tauri::State<'_, $crate::transport::Bus>,
        ) -> ::std::result::Result<$ret, ::std::string::String> {
            let $bind = $mint;
            bus.execute($cmd).await.map_err(|e| e.to_string())?;
            let $out = bus
                .query($query)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| ($missing).to_string())?;
            ::std::result::Result::Ok($map)
        }
    };
}

/// Expand to the `tauri_specta::collect_commands![...]` array of every desktop IPC command.
/// Declarative (not `inventory`): tauri needs the handler idents at compile time. Lists
/// the macro-generated domain shims (`transport::domain`) alongside the hand-written
/// host/shell and transport-resident shims.
///
/// Accepts optional runtime-specific commands as arguments -- handlers that cannot be
/// registered on the test `MockRuntime`. Pass them positionally:
/// - `collect_transport!($crate::bridge::daemon_connect)` in production (`lib.rs`).
/// - `collect_transport!()` in the command-contract test (omits `daemon_connect`).
///
/// Stays hand-written (NOT macro-generated), listed here:
/// - host/shell (no domain store): `window_host`, `diag`, `bridge`, `menu`,
///   `supervisor`, `orchestrator_host` status/health, `store` (file-backed prefs +
///   session registry, off-bus).
/// - transport-resident: the `surface_channel` shims register/attach a per-surface
///   `tauri::ipc::Channel` and the off-bus client messages write straight to the
///   runtime port (a `Channel` is a tauri object that cannot live in the core).
macro_rules! collect_transport {
    ( $( $runtime_cmd:path ),* $(,)? ) => {
        tauri::generate_handler![
            $( $runtime_cmd, )*
            $crate::bridge::daemon_send,
            $crate::bridge::daemon_disconnect,
            $crate::diag::log_forward,
            $crate::store::pref_get,
            $crate::store::pref_set,
            $crate::store::registry_get,
            $crate::store::registry_set,
            $crate::store::registry_remove,
            $crate::store::registry_list,
            $crate::supervisor::daemon_ensure,
            $crate::orchestrator_host::orchestrator_status,
            $crate::orchestrator_host::service_health,
            $crate::window_host::window_open,
            $crate::window_host::window_focus,
            $crate::window_host::window_close,
            $crate::menu::command_center_set_leader,
            $crate::transport::surface::surface_channel,
            $crate::transport::surface::surface_channel_send,
            $crate::transport::surface::surface_spawn,
            $crate::transport::surface::surface_close,
            $crate::transport::surface::surface_detach,
            $crate::transport::logs::log_subscribe,
            $crate::transport::logs::log_unsubscribe,
            $crate::transport::project::project_create,
            $crate::transport::project::project_list,
            $crate::transport::project::project_rename,
            $crate::transport::project::project_archive,
            $crate::transport::project::project_delete,
            $crate::transport::project::project_reorder,
            $crate::transport::project::project_move,
            $crate::transport::project::project_get,
            $crate::transport::project::project_search,
            $crate::transport::project::project_restore,
            $crate::transport::project::project_duplicate,
            $crate::transport::project::project_pin,
            $crate::transport::project::project_unpin,
            $crate::transport::project::project_stop_surfaces,
            $crate::transport::workspace::workspace_create,
            $crate::transport::workspace::workspace_list,
            $crate::transport::workspace::workspace_rename,
            $crate::transport::workspace::workspace_reorder,
            $crate::transport::workspace::workspace_delete,
            $crate::transport::workspace::workspace_get,
            $crate::transport::workspace::workspace_archive,
            $crate::transport::workspace::workspace_restore,
            $crate::transport::workspace::workspace_pin,
            $crate::transport::workspace::workspace_unpin,
            $crate::transport::workspace::workspace_stop_surfaces,
            $crate::transport::surface::surface_get,
            $crate::transport::surface::surface_list_by_session,
            $crate::transport::surface::surface_list_resumable,
            $crate::transport::surface::surface_find_by_placement,
            $crate::transport::surface::surface_stop,
            $crate::transport::surface::surface_reconcile,
            $crate::transport::session::session_list,
            $crate::transport::session::session_create,
            $crate::transport::session::session_rename,
            $crate::transport::session::session_archive,
            $crate::transport::session::session_delete,
            $crate::transport::session::session_reorder,
            $crate::transport::session::session_layout_set,
            $crate::transport::session::session_layout_get,
            $crate::transport::session::session_get,
            $crate::transport::session::session_list_all,
            $crate::transport::session::session_get_launch_spec,
            $crate::transport::session::session_search,
            $crate::transport::session::session_launch,
            $crate::transport::session::session_apply_launch_spec,
            $crate::transport::session::session_move,
            $crate::transport::session::session_duplicate,
            $crate::transport::session::session_pin,
            $crate::transport::session::session_unpin,
            $crate::transport::session::session_restore,
            $crate::transport::session::session_stop_surfaces,
            $crate::transport::command::command_list,
            $crate::transport::command::command_create,
            $crate::transport::command::command_get,
            $crate::transport::command::command_delete,
            $crate::transport::command::command_rename,
            $crate::transport::command::command_edit,
            $crate::transport::command::command_pin,
            $crate::transport::command::command_unpin,
            $crate::transport::command::command_duplicate,
            $crate::transport::command::command_seed,
            $crate::transport::settings::setting_get,
            $crate::transport::settings::setting_set,
            $crate::transport::settings::setting_list,
            $crate::transport::settings::setting_reset,
            $crate::transport::settings::setting_resolve,
            $crate::transport::settings::settings_resolve,
            $crate::transport::settings::profile_get_active,
            $crate::transport::settings::profile_list,
            $crate::transport::settings::profile_create,
            $crate::transport::settings::profile_activate,
            $crate::transport::settings::profile_rename,
            $crate::transport::settings::profile_duplicate,
            $crate::transport::settings::profile_discard,
            $crate::transport::settings::profile_export,
            $crate::transport::settings::profile_import,
            $crate::transport::settings::theme_get_active,
            $crate::transport::settings::theme_list,
            $crate::transport::settings::theme_activate,
            $crate::transport::settings::theme_discard,
            $crate::transport::settings::theme_export,
            $crate::transport::settings::theme_import,
            $crate::transport::settings::keybinding_list,
            $crate::transport::settings::keybinding_rebind,
            $crate::transport::settings::keybinding_reset,
            $crate::transport::settings::keybinding_reset_all,
            $crate::transport::settings::keybinding_resolve,
            $crate::transport::settings::config_reload,
            $crate::transport::notification::notifications_list,
            $crate::transport::notification::notification_list_unread,
            $crate::transport::notification::notification_count_unread,
            $crate::transport::notification::notification_mark_read,
            $crate::transport::notification::notification_mark_all_read,
            $crate::transport::notification::notification_disregard,
            $crate::transport::notification::notification_disregard_all,
            $crate::transport::notification::notification_snooze,
            $crate::transport::notification::notification_prune,
            $crate::transport::notification::notification_record,
            $crate::transport::template::launch_template_create,
            $crate::transport::template::launch_template_list,
            $crate::transport::template::launch_template_get,
            $crate::transport::template::launch_template_discard,
            $crate::transport::template::launch_template_apply_spec,
            $crate::transport::template::template_list,
            $crate::transport::template::template_get,
            $crate::transport::template::template_import,
            $crate::transport::template::template_export,
            $crate::transport::template::template_discard,
            $crate::transport::template::template_pin,
            $crate::transport::template::template_unpin,
            $crate::transport::logs::log_list,
            $crate::transport::logs::log_tail,
        ]
    };
}

pub(crate) use {collect_transport, transport_command, transport_create, transport_query};

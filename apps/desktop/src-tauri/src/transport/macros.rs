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
//! and transport-resident shims (window/file/log/bridge/menu/supervisor/gate/store, the
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

/// Expand to the `tauri::generate_handler![...]` array of every desktop IPC command.
/// Declarative (not `inventory`): tauri needs the handler idents at compile time. Lists
/// the macro-generated domain shims (`transport::domain`) alongside the hand-written
/// host/shell and transport-resident shims.
///
/// Stays hand-written (NOT macro-generated), listed here:
/// - host/shell (no domain store): `window_host`, `files`, `diag`, `bridge`, `menu`,
///   `supervisor`, `orchestrator_host` status/health, `store` (file-backed prefs +
///   session registry, design D6 / off-bus).
/// - transport-resident: every `surface_*` shim -- they register/attach a per-surface
///   `tauri::ipc::Channel` and the off-bus input/resize endpoints write straight to the
///   runtime port (a `Channel` is a tauri object that cannot live in the core).
/// - non-mechanical domain: `settings_host::*` (wire `scope`+`projectId` -> `SettingScope`
///   parse and JSON value <-> string conversion) and `notification_host::notifications_list`
///   (fixed-page query that lives with the notification sink/builders).
macro_rules! collect_transport {
    () => {
        tauri::generate_handler![
            // -- host / shell (out of CQS scope) --
            $crate::bridge::daemon_connect,
            $crate::bridge::daemon_send,
            $crate::bridge::daemon_disconnect,
            $crate::files::file_size,
            $crate::files::file_read,
            $crate::files::list_log_files,
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
            // -- surface I/O (transport-resident: ipc::Channel + off-bus runtime) --
            $crate::surface_host::surface_create,
            $crate::surface_host::surface_spawn,
            $crate::surface_host::surface_close,
            $crate::surface_host::surface_input,
            $crate::surface_host::surface_resize,
            $crate::surface_host::surface_detach,
            // -- domain (macro-generated + the hand-written list-diff creates) --
            $crate::transport::domain::project_create,
            $crate::transport::domain::project_list,
            $crate::transport::domain::project_rename,
            $crate::transport::domain::project_archive,
            $crate::transport::domain::project_delete,
            $crate::transport::domain::project_reorder,
            $crate::transport::domain::project_move,
            $crate::transport::domain::workspace_create,
            $crate::transport::domain::workspace_list,
            $crate::transport::domain::workspace_rename,
            $crate::transport::domain::workspace_reorder,
            $crate::transport::domain::workspace_delete,
            $crate::transport::domain::session_list,
            $crate::transport::domain::session_create,
            $crate::transport::domain::session_rename,
            $crate::transport::domain::session_archive,
            $crate::transport::domain::session_delete,
            $crate::transport::domain::session_reorder,
            $crate::transport::domain::session_layout_set,
            $crate::transport::domain::session_layout_get,
            $crate::transport::domain::command_list,
            $crate::transport::domain::command_create,
            $crate::transport::domain::command_get,
            $crate::transport::domain::command_delete,
            // -- settings (non-mechanical: scope parse + JSON value<->string) --
            $crate::settings_host::setting_get,
            $crate::settings_host::setting_set,
            $crate::settings_host::setting_list,
            // -- notifications (fixed-page query beside the notification sink) --
            $crate::notification_host::notifications_list,
        ]
    };
}

pub(crate) use {collect_transport, transport_command, transport_create, transport_query};

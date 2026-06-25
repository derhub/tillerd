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

/// Generate a `#[tauri::command]` shim for a client stream subscription. Takes the
/// renderer-provided `tauri::ipc::Channel<Vec<u8>>` plus wire params and exposes a
/// `$mk` closure that mints a per-stream `ChannelSink` (the host adapter over the
/// registry) bound to the channel + app handle. The body wires that sink into a
/// subscribe command through the bus. The bus binding (`bus = $bus`) and the wire
/// params are in scope. Generic over the tauri runtime because it threads an `AppHandle`
/// for the lifecycle events the sink emits. `$mk` may be called more than once (it
/// clones the channel + app handle per sink), so a revisit-then-respawn path can
/// register a fresh sink each time.
macro_rules! transport_subscribe {
    (
        $(#[$meta:meta])*
        $vis:vis $name:ident ( $( $param:ident : $ty:ty ),* $(,)? ) -> $ret:ty,
            bus = $bus:ident,
            sink = $mk:ident,
            $body:block
    ) => {
        $(#[$meta])*
        #[tauri::command]
        #[specta::specta]
        #[allow(clippy::too_many_arguments)] // generated transport shim; arg count mirrors the wire command
        $vis async fn $name<R: tauri::Runtime>(
            app: tauri::AppHandle<R>,
            $bus: tauri::State<'_, $crate::transport::Bus>,
            channel: tauri::ipc::Channel<::std::vec::Vec<u8>>,
            $( $param: $ty, )*
        ) -> ::std::result::Result<$ret, ::std::string::String> {
            let $mk = || -> ::std::sync::Arc<dyn ::orchestrator::app::surface::SurfaceSink> {
                ::std::sync::Arc::new(
                    $crate::transport::sink::ChannelSink::for_channel(channel.clone(), app.clone()),
                )
            };
            $body
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
/// - host/shell (no domain store): `window_host`, `files`, `diag`, `bridge`, `menu`,
///   `supervisor`, `orchestrator_host` status/health, `store` (file-backed prefs +
///   session registry, off-bus).
/// - transport-resident: every `surface_*` shim -- they register/attach a per-surface
///   `tauri::ipc::Channel` and the off-bus input/resize endpoints write straight to the
///   runtime port (a `Channel` is a tauri object that cannot live in the core).
/// - non-mechanical domain: `notification_host::notifications_list` (fixed-page query that
///   lives with the notification sink/builders).
macro_rules! collect_transport {
    ( $( $runtime_cmd:path ),* $(,)? ) => {
        tauri::generate_handler![
            $( $runtime_cmd, )*
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
            $crate::surface_host::surface_create,
            $crate::surface_host::surface_channel,
            $crate::surface_host::surface_channel_send_cmd,
            $crate::surface_host::surface_spawn,
            $crate::surface_host::surface_close,
            $crate::surface_host::surface_input,
            $crate::surface_host::surface_resize,
            $crate::surface_host::surface_detach,
            $crate::transport::domain::project_create,
            $crate::transport::domain::project_list,
            $crate::transport::domain::project_rename,
            $crate::transport::domain::project_archive,
            $crate::transport::domain::project_delete,
            $crate::transport::domain::project_reorder,
            $crate::transport::domain::project_move,
            $crate::transport::domain::project_get,
            $crate::transport::domain::project_search,
            $crate::transport::domain::project_restore,
            $crate::transport::domain::project_duplicate,
            $crate::transport::domain::project_pin,
            $crate::transport::domain::project_unpin,
            $crate::transport::domain::project_stop_surfaces,
            $crate::transport::domain::workspace_create,
            $crate::transport::domain::workspace_list,
            $crate::transport::domain::workspace_rename,
            $crate::transport::domain::workspace_reorder,
            $crate::transport::domain::workspace_delete,
            $crate::transport::domain::workspace_get,
            $crate::transport::domain::workspace_archive,
            $crate::transport::domain::workspace_restore,
            $crate::transport::domain::workspace_pin,
            $crate::transport::domain::workspace_unpin,
            $crate::transport::domain::workspace_stop_surfaces,
            $crate::transport::domain::surface_get,
            $crate::transport::domain::surface_list_by_session,
            $crate::transport::domain::surface_list_resumable,
            $crate::transport::domain::surface_find_by_placement,
            $crate::transport::domain::surface_stop,
            $crate::transport::domain::surface_reconcile,
            $crate::transport::domain::session_list,
            $crate::transport::domain::session_create,
            $crate::transport::domain::session_rename,
            $crate::transport::domain::session_archive,
            $crate::transport::domain::session_delete,
            $crate::transport::domain::session_reorder,
            $crate::transport::domain::session_layout_set,
            $crate::transport::domain::session_layout_get,
            $crate::transport::domain::session_get,
            $crate::transport::domain::session_list_all,
            $crate::transport::domain::session_get_launch_spec,
            $crate::transport::domain::session_search,
            $crate::transport::domain::session_launch,
            $crate::transport::domain::session_apply_launch_spec,
            $crate::transport::domain::session_move,
            $crate::transport::domain::session_duplicate,
            $crate::transport::domain::session_pin,
            $crate::transport::domain::session_unpin,
            $crate::transport::domain::session_restore,
            $crate::transport::domain::session_stop_surfaces,
            $crate::transport::domain::command_list,
            $crate::transport::domain::command_create,
            $crate::transport::domain::command_get,
            $crate::transport::domain::command_delete,
            $crate::transport::domain::command_rename,
            $crate::transport::domain::command_edit,
            $crate::transport::domain::command_pin,
            $crate::transport::domain::command_unpin,
            $crate::transport::domain::command_duplicate,
            $crate::transport::domain::command_seed,
            $crate::transport::domain::setting_get,
            $crate::transport::domain::setting_set,
            $crate::transport::domain::setting_list,
            $crate::transport::domain::setting_reset,
            $crate::transport::domain::setting_resolve,
            $crate::transport::domain::settings_resolve,
            $crate::transport::domain::profile_get_active,
            $crate::transport::domain::profile_list,
            $crate::transport::domain::profile_create,
            $crate::transport::domain::profile_activate,
            $crate::transport::domain::profile_rename,
            $crate::transport::domain::profile_duplicate,
            $crate::transport::domain::profile_discard,
            $crate::transport::domain::profile_export,
            $crate::transport::domain::profile_import,
            $crate::transport::domain::theme_get_active,
            $crate::transport::domain::theme_list,
            $crate::transport::domain::theme_activate,
            $crate::transport::domain::theme_discard,
            $crate::transport::domain::theme_export,
            $crate::transport::domain::theme_import,
            $crate::transport::domain::keybinding_list,
            $crate::transport::domain::keybinding_rebind,
            $crate::transport::domain::keybinding_reset,
            $crate::transport::domain::keybinding_reset_all,
            $crate::transport::domain::keybinding_resolve,
            $crate::transport::domain::config_reload,
            $crate::notification_host::notifications_list,
            $crate::transport::domain::notification_list_unread,
            $crate::transport::domain::notification_count_unread,
            $crate::transport::domain::notification_mark_read,
            $crate::transport::domain::notification_mark_all_read,
            $crate::transport::domain::notification_disregard,
            $crate::transport::domain::notification_disregard_all,
            $crate::transport::domain::notification_snooze,
            $crate::transport::domain::notification_prune,
            $crate::transport::domain::notification_record,
            $crate::transport::domain::launch_template_create,
            $crate::transport::domain::launch_template_list,
            $crate::transport::domain::launch_template_get,
            $crate::transport::domain::launch_template_discard,
            $crate::transport::domain::launch_template_apply_spec,
            $crate::transport::domain::template_list,
            $crate::transport::domain::template_get,
            $crate::transport::domain::template_import,
            $crate::transport::domain::template_export,
            $crate::transport::domain::template_discard,
            $crate::transport::domain::template_pin,
            $crate::transport::domain::template_unpin,
            $crate::transport::domain::log_list,
            $crate::transport::domain::log_tail,
        ]
    };
}

pub(crate) use {
    collect_transport, transport_command, transport_create, transport_query, transport_subscribe,
};

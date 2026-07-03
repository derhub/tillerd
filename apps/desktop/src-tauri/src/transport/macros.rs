macro_rules! transport_command {
    (
        $(#[$meta:meta])*
        $name:ident ( $( $param:ident : $ty:ty ),* $(,)? ) => $build:expr
    ) => {
        $(#[$meta])*
        #[tauri::command]
        #[specta::specta]
        #[allow(clippy::too_many_arguments)]
        pub async fn $name(
            $( $param: $ty, )*
            bus: tauri::State<'_, $crate::transport::Bus>,
        ) -> ::std::result::Result<(), ::std::string::String> {
            bus.execute($build).await.map_err(|e| e.to_string())
        }
    };
}

macro_rules! transport_query {
    (
        $(#[$meta:meta])*
        $name:ident ( $( $param:ident : $ty:ty ),* $(,)? ) -> $ret:ty
            => $query:expr , | $out:ident | $map:expr
    ) => {
        $(#[$meta])*
        #[tauri::command]
        #[specta::specta]
        #[allow(clippy::too_many_arguments)]
        pub async fn $name(
            $( $param: $ty, )*
            bus: tauri::State<'_, $crate::transport::Bus>,
        ) -> ::std::result::Result<$ret, ::std::string::String> {
            let $out = bus.query($query).await.map_err(|e| e.to_string())?;
            ::std::result::Result::Ok($map)
        }
    };
}

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
        #[allow(clippy::too_many_arguments)]
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
    // Create with a non-fatal tail: after the read-back, `tail` runs with the mapped value bound
    // (and `bus` in scope) for follow-up dispatch whose failure must not invalidate the create
    // (launch a spec, announce a notable). The tail block owns its own error handling.
    (
        $(#[$meta:meta])*
        $name:ident ( $( $param:ident : $ty:ty ),* $(,)? ) -> $ret:ty {
            let $bind:ident = $mint:expr;
            execute: $cmd:expr,
            read_back: $query:expr,
            map: | $out:ident | $map:expr,
            missing: $missing:expr,
            tail: | $created:ident, $busid:ident | $tail:block $(,)?
        }
    ) => {
        $(#[$meta])*
        #[tauri::command]
        #[specta::specta]
        #[allow(clippy::too_many_arguments)]
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
            let result = $map;
            {
                let $created = &result;
                let $busid = &bus;
                $tail
            }
            ::std::result::Result::Ok(result)
        }
    };
}

macro_rules! collect_transport {
    ( $( $runtime_cmd:path ),* $(,)? ) => {
        tauri::generate_handler![
            $( $runtime_cmd, )*
            $crate::orchestrator_host::orchestrator_status,
            $crate::orchestrator_host::service_health,
            $crate::window_host::window_open,
            $crate::window_host::window_focus,
            $crate::window_host::window_close,
            $crate::menu::command_center_set_leader,
            $crate::transport::surface::surface_resolve_or_spawn,
            $crate::transport::surface::surface_channel,
            $crate::transport::surface::surface_channel_send,
            $crate::transport::surface::surface_channel_close,
            $crate::transport::surface::surface_spawn,
            $crate::transport::surface::surface_close,
            $crate::transport::surface::surface_detach,
            $crate::transport::logs::log_channel,
            $crate::transport::logs::log_channel_close,
            $crate::transport::logs::logs_changed_channel,
            $crate::transport::logs::logs_changed_channel_close,
            $crate::transport::notification::notification_channel,
            $crate::transport::notification::notification_channel_close,
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
            $crate::transport::workspace::workspace_activity,
            $crate::transport::workspace::workspace_rename,
            $crate::transport::workspace::workspace_reorder,
            $crate::transport::workspace::workspace_delete,
            $crate::transport::workspace::workspace_get,
            $crate::transport::workspace::workspace_archive,
            $crate::transport::workspace::workspace_restore,
            $crate::transport::workspace::workspace_pin,
            $crate::transport::workspace::workspace_unpin,
            $crate::transport::workspace::workspace_stop_surfaces,
            $crate::transport::surface::surface_status_channel,
            $crate::transport::surface::surface_status_channel_close,
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

macro_rules! domain_channel {
    (
        pub open $open_name:ident ( $req_ty:ty ),
        pub send $send_name:ident ( $msg_ty:ty ),
        pub close $close_name:ident ( $close_ty:ty )
    ) => {
        #[tauri::command]
        #[specta::specta]
        pub async fn $open_name<R: tauri::Runtime>(
            app: tauri::AppHandle<R>,
            bus: tauri::State<'_, $crate::transport::Bus>,
            channel: tauri::ipc::Channel<::std::vec::Vec<u8>>,
            req: $req_ty,
        ) -> ::std::result::Result<(), ::std::string::String> {
            use orchestrator::shared::domain_channel::OpenDomainChannel;
            let sink = ::std::sync::Arc::new($crate::transport::surface::ChannelSink::for_channel(
                channel, app,
            ));
            req.handle(bus.cx(), sink).await.map_err(|e| e.to_string())
        }

        #[tauri::command]
        #[specta::specta]
        pub async fn $send_name(
            bus: tauri::State<'_, $crate::transport::Bus>,
            key: ::std::string::String,
            msg: $msg_ty,
        ) -> ::std::result::Result<(), ::std::string::String> {
            use orchestrator::shared::domain_channel::DomainChannelMessage;
            msg.handle(bus.cx(), &key).await.map_err(|e| e.to_string())
        }

        #[tauri::command]
        #[specta::specta]
        pub async fn $close_name(
            bus: tauri::State<'_, $crate::transport::Bus>,
            req: $close_ty,
        ) -> ::std::result::Result<(), ::std::string::String> {
            use orchestrator::shared::domain_channel::CloseDomainChannel;
            req.handle(bus.cx()).await.map_err(|e| e.to_string())
        }
    };

    (
        pub open $open_name:ident ( $req_ty:ty ),
        pub close $close_name:ident ( $close_ty:ty )
    ) => {
        #[tauri::command]
        #[specta::specta]
        pub async fn $open_name<R: tauri::Runtime>(
            app: tauri::AppHandle<R>,
            bus: tauri::State<'_, $crate::transport::Bus>,
            channel: tauri::ipc::Channel<::std::vec::Vec<u8>>,
            req: $req_ty,
        ) -> ::std::result::Result<(), ::std::string::String> {
            use orchestrator::shared::domain_channel::OpenDomainChannel;
            let sink = ::std::sync::Arc::new($crate::transport::surface::ChannelSink::for_channel(
                channel, app,
            ));
            req.handle(bus.cx(), sink).await.map_err(|e| e.to_string())
        }

        #[tauri::command]
        #[specta::specta]
        pub async fn $close_name(
            bus: tauri::State<'_, $crate::transport::Bus>,
            req: $close_ty,
        ) -> ::std::result::Result<(), ::std::string::String> {
            use orchestrator::shared::domain_channel::CloseDomainChannel;
            req.handle(bus.cx()).await.map_err(|e| e.to_string())
        }
    };
}

pub(crate) use {
    collect_transport, domain_channel, transport_command, transport_create, transport_query,
};

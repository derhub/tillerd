use orchestrator::app::surface::{
    CloseSurface, DetachSurface, FindSurfaceByPlacement, GetSurfaceById, ListResumableSurfaces,
    ListSurfacesBySession, ReconcileSurfaces, ResolveOrSpawnSurface, SpawnCommandRef, StopSurface,
    SurfaceView, SwapPlacement,
};
use tauri::{AppHandle, Runtime};

use crate::transport::macros::{
    domain_channel, transport_command, transport_create, transport_query,
};

pub struct ChannelSink {
    channel: tauri::ipc::Channel<Vec<u8>>,
}

impl ChannelSink {
    pub fn for_channel(
        channel: tauri::ipc::Channel<Vec<u8>>,
        _app: AppHandle<impl Runtime>,
    ) -> Self {
        Self { channel }
    }
}

impl orchestrator::shared::domain_channel::DomainChannelSink for ChannelSink {
    fn emit(&self, event: &orchestrator::shared::domain_channel::DomainChannelEvent<'_>) {
        use orchestrator::shared::domain_channel::DomainChannelEvent;
        let mut payload = Vec::new();
        match event {
            DomainChannelEvent::Bytes(bytes) => {
                payload.push(0x00);
                payload.extend_from_slice(bytes);
            }
            DomainChannelEvent::Status(status) => {
                payload.push(0x01);
                payload.extend_from_slice(status.as_bytes());
            }
            DomainChannelEvent::Exit(qualifier) => {
                payload.push(0x02);
                payload.extend_from_slice(qualifier.as_bytes());
            }
            DomainChannelEvent::Error(reason) => {
                payload.push(0x03);
                payload.extend_from_slice(reason.as_bytes());
            }
        }
        let _ = self.channel.send(payload);
    }
}

domain_channel! {
    pub open surface_channel(orchestrator::app::surface::OpenSurfaceChannel),
    pub send surface_channel_send(orchestrator::app::surface::SurfaceClientMsg),
    pub close surface_channel_close(orchestrator::app::surface::CloseSurfaceChannel)
}

domain_channel! {
    pub open surface_status_channel(orchestrator::app::surface::OpenSurfaceStatusChannel),
    pub close surface_status_channel_close(orchestrator::app::surface::CloseSurfaceStatusChannel)
}

transport_query!(
    surface_resolve_or_spawn(session: String, placement: String, cwd: Option<String>, cols: Option<u16>, rows: Option<u16>) -> SurfaceView
        => ResolveOrSpawnSurface { session, placement, cwd, cols, rows },
        |surface| surface
);

transport_query!(
    surface_get(id: String) -> Option<SurfaceView>
        => GetSurfaceById { id },
        |surface| surface
);

transport_query!(
    surface_list_by_session(session: String, limit: Option<u32>, offset: Option<u32>, after: Option<String>) -> Vec<SurfaceView>
        => ListSurfacesBySession { session, limit, offset, after },
        |listing| listing.items
);

transport_query!(
    surface_list_resumable() -> Vec<SurfaceView>
        => ListResumableSurfaces,
        |surfaces| surfaces
);

transport_query!(
    surface_find_by_placement(session: String, placement: String) -> Option<SurfaceView>
        => FindSurfaceByPlacement { session, placement },
        |surface| surface
);

transport_command!(surface_stop(id: String) => StopSurface { id });

transport_command!(
    surface_swap_placement(session: String, placement_a: String, placement_b: String)
        => SwapPlacement { session, placement_a, placement_b }
);

transport_command!(surface_reconcile() => ReconcileSurfaces);

transport_create!(
    /// Spawn a terminal surface: mint a placement, spawn, read the surface back by
    /// placement, then announce `SurfaceStarted` via the non-fatal notable tail.
    /// `command`, when given (library ref or inline), diverges the session's launch
    /// spec so the surface survives a reconcile.
    surface_spawn(session_id: String, command: Option<SpawnCommandRef>) -> String {
        let placement = uuid::Uuid::new_v4().to_string();
        execute: {
            let (command_library_ref, command_executable, command_args) = command
                .map(SpawnCommandRef::into_dto_fields)
                .unwrap_or((None, None, Vec::new()));
            orchestrator::app::surface::SpawnSurface {
                session: session_id.clone(),
                kind: "terminal".to_string(),
                cwd: None,
                placement: Some(placement.clone()),
                cols: None,
                rows: None,
                command_library_ref,
                command_executable,
                command_args,
            }
        },
        read_back: orchestrator::app::surface::FindSurfaceByPlacement {
            session: session_id.clone(),
            placement,
        },
        map: |surface| surface.id,
        missing: "surface vanished after spawn",
        tail: |created, bus| {
            let _ = bus
                .execute_notable(orchestrator::app::notification::SurfaceStarted {
                    surface_id: created.clone(),
                    session_id,
                    ts: crate::transport::notification::now_ms(),
                })
                .await;
        },
    }
);

transport_command!(surface_close(id: String) => CloseSurface { id });
transport_command!(surface_detach(id: String) => DetachSurface { id });

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use orchestrator::app::surface::{CloseSurfaceChannel, OpenSurfaceChannel, SurfaceClientMsg};
    use orchestrator::boot::{test_ctx_with_probe, TestRuntimeProbe};
    use orchestrator::shared::domain_channel::{
        CloseDomainChannel, DomainChannelEvent, DomainChannelMessage, DomainChannelSink,
        OpenDomainChannel,
    };
    use orchestrator::shared::Bus;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    use super::*;

    #[derive(Default, Clone)]
    struct BusSpans(Arc<Mutex<usize>>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for BusSpans {
        fn on_new_span(
            &self,
            attrs: &tracing::span::Attributes<'_>,
            _id: &tracing::span::Id,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let name = attrs.metadata().name();
            if name == "command" || name == "query" {
                *self.0.lock().unwrap() += 1;
            }
        }
    }

    async fn bus_with_probe() -> (Bus<orchestrator::Ctx>, TestRuntimeProbe) {
        let (cx, probe) = test_ctx_with_probe().await.unwrap();
        (Bus::new(cx), probe)
    }

    #[tokio::test]
    async fn input_send_writes_the_bytes_to_the_runtime() {
        let (bus, probe) = bus_with_probe().await;

        SurfaceClientMsg::Input {
            bytes: b"ls\n".to_vec(),
        }
        .handle(bus.cx(), "sf_1")
        .await
        .unwrap();

        assert_eq!(probe.inputs(), vec![("sf_1".to_owned(), b"ls\n".to_vec())]);
    }

    #[tokio::test]
    async fn input_send_opens_no_bus_span() {
        let spans = BusSpans::default();
        let _guard = tracing_subscriber::registry()
            .with(spans.clone())
            .set_default();

        let (bus, _probe) = bus_with_probe().await;
        SurfaceClientMsg::Input {
            bytes: b"secret".to_vec(),
        }
        .handle(bus.cx(), "sf_1")
        .await
        .unwrap();

        assert_eq!(*spans.0.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn resize_send_reaches_the_runtime() {
        let (bus, probe) = bus_with_probe().await;

        SurfaceClientMsg::Resize {
            cols: 120,
            rows: 40,
        }
        .handle(bus.cx(), "sf_1")
        .await
        .unwrap();

        assert_eq!(probe.resizes(), vec![("sf_1".to_owned(), 120, 40)]);
    }

    #[tokio::test]
    async fn output_frames_reach_the_registered_sink() {
        let (bus, _probe) = bus_with_probe().await;
        let seen: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
        let recorder = seen.clone();
        struct ByteRecorder(Arc<Mutex<Vec<Vec<u8>>>>);
        impl DomainChannelSink for ByteRecorder {
            fn emit(&self, event: &DomainChannelEvent<'_>) {
                if let DomainChannelEvent::Bytes(b) = event {
                    self.0.lock().unwrap().push(b.to_vec());
                }
            }
        }

        OpenSurfaceChannel {
            surface_id: "sf_1".to_owned(),
        }
        .handle(bus.cx(), Arc::new(ByteRecorder(recorder)))
        .await
        .unwrap();

        bus.cx()
            .domain_channel_sinks()
            .dispatch("surface://sf_1", |s| {
                s.emit(&DomainChannelEvent::Bytes(b"out"))
            });

        assert_eq!(seen.lock().unwrap().as_slice(), [b"out".to_vec()]);
    }

    #[tokio::test]
    async fn two_inputs_arrive_in_order() {
        let (bus, probe) = bus_with_probe().await;

        SurfaceClientMsg::Input {
            bytes: b"first".to_vec(),
        }
        .handle(bus.cx(), "sf_1")
        .await
        .unwrap();
        SurfaceClientMsg::Input {
            bytes: b"second".to_vec(),
        }
        .handle(bus.cx(), "sf_1")
        .await
        .unwrap();

        assert_eq!(
            probe.inputs(),
            vec![
                ("sf_1".to_owned(), b"first".to_vec()),
                ("sf_1".to_owned(), b"second".to_vec()),
            ]
        );
    }

    #[tokio::test]
    async fn close_send_unsubscribes_through_the_bus() {
        let (bus, _probe) = bus_with_probe().await;
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let recorder = seen.clone();
        struct Recorder(Arc<Mutex<Vec<String>>>);
        impl DomainChannelSink for Recorder {
            fn emit(&self, event: &DomainChannelEvent<'_>) {
                if let DomainChannelEvent::Bytes(_) = event {
                    self.0.lock().unwrap().push("sf_1".to_owned());
                }
            }
        }

        OpenSurfaceChannel {
            surface_id: "sf_1".to_owned(),
        }
        .handle(bus.cx(), Arc::new(Recorder(recorder)))
        .await
        .unwrap();

        bus.cx()
            .domain_channel_sinks()
            .dispatch("surface://sf_1", |s| {
                s.emit(&DomainChannelEvent::Bytes(b"pre"))
            });
        assert_eq!(seen.lock().unwrap().as_slice(), ["sf_1".to_owned()]);

        CloseSurfaceChannel {
            surface_id: "sf_1".to_owned(),
        }
        .handle(bus.cx())
        .await
        .unwrap();

        bus.cx()
            .domain_channel_sinks()
            .dispatch("surface://sf_1", |s| {
                s.emit(&DomainChannelEvent::Bytes(b"post"))
            });
        assert_eq!(seen.lock().unwrap().as_slice(), ["sf_1".to_owned()]);
    }

    fn assert_keys(value: &serde_json::Value, expected: &[&str]) {
        let obj = value.as_object().expect("response serializes to an object");
        let mut got: Vec<&str> = obj.keys().map(String::as_str).collect();
        got.sort_unstable();
        let mut want = expected.to_vec();
        want.sort_unstable();
        assert_eq!(got, want, "response keys drifted from the SDK contract");
    }

    #[test]
    fn surface_response_matches_sdk_surface_shape() {
        let s = SurfaceView {
            id: "s".into(),
            session_id: "sess".into(),
            kind: "terminal".into(),
            cwd: None,
            status: "live".into(),
            placement: Some("main".into()),
            spawned_at: Some(1_700_000_000_000),
        };
        assert_keys(
            &serde_json::to_value(s).unwrap(),
            &[
                "id",
                "sessionId",
                "kind",
                "cwd",
                "status",
                "placement",
                "spawnedAt",
            ],
        );
    }
}


use std::time::{SystemTime, UNIX_EPOCH};

use orchestrator::app::surface::{
    attach_surface, resize_surface, send_surface_input, CloseSurface, DetachSurface,
    FindSurfaceByPlacement, GetSurfaceById, ListResumableSurfaces, ListSurfacesBySession,
    ReconcileSurfaces, SpawnSurface, StopSurface, SubscribeSurface, SurfaceEvent, SurfaceSink,
    SurfaceView, UnsubscribeSurface,
};
use orchestrator::app::notification::SurfaceStarted;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};

use crate::transport::Bus;
use crate::transport::macros::{transport_command, transport_create, transport_query};

pub const STATUS_EVENT: &str = "surface://status";
pub const EXIT_EVENT: &str = "surface://exit";
pub const ERROR_EVENT: &str = "surface:error";

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "surface://status")]
#[serde(rename_all = "camelCase")]
pub struct SurfaceStatusPayload {
    pub surface_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "surface://exit")]
#[serde(rename_all = "camelCase")]
pub struct SurfaceExitPayload {
    pub surface_id: String,
    pub qualifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "surface:error")]
#[serde(rename_all = "camelCase")]
pub struct SurfaceErrorPayload {
    pub surface_id: String,
    pub reason: String,
}

pub struct ChannelSink<R: Runtime> {
    channel: tauri::ipc::Channel<Vec<u8>>,
    app: AppHandle<R>,
}

impl<R: Runtime> ChannelSink<R> {
    pub fn for_channel(channel: tauri::ipc::Channel<Vec<u8>>, app: AppHandle<R>) -> Self {
        Self { channel, app }
    }
}

impl<R: Runtime> SurfaceSink for ChannelSink<R> {
    fn emit(&self, surface: &str, event: &SurfaceEvent<'_>) {
        match event {
            SurfaceEvent::Bytes(bytes) => {
                let _ = self.channel.send(bytes.to_vec());
            }
            SurfaceEvent::Status(status) => {
                let _ = self.app.emit(
                    STATUS_EVENT,
                    serde_json::json!({ "surfaceId": surface, "status": status }),
                );
            }
            SurfaceEvent::Exit(qualifier) => {
                let _ = self.app.emit(
                    EXIT_EVENT,
                    serde_json::json!({ "surfaceId": surface, "qualifier": qualifier }),
                );
            }
            SurfaceEvent::Error(reason) => {
                let _ = self.app.emit(
                    ERROR_EVENT,
                    serde_json::json!({ "surfaceId": surface, "reason": reason }),
                );
            }
        }
    }
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SurfaceClientMsg {
    Input { bytes: Vec<u8> },
    Resize { cols: u16, rows: u16 },
    Close,
}

pub async fn route_surface_send(
    bus: &Bus,
    key: String,
    msg: SurfaceClientMsg,
) -> Result<(), String> {
    match msg {
        SurfaceClientMsg::Input { bytes } => send_surface_input(bus.cx(), &key, &bytes)
            .await
            .map_err(|e| e.to_string()),
        SurfaceClientMsg::Resize { cols, rows } => resize_surface(bus.cx(), &key, cols, rows)
            .await
            .map_err(|e| e.to_string()),
        SurfaceClientMsg::Close => bus
            .execute(UnsubscribeSurface { surface_id: key })
            .await
            .map_err(|e| e.to_string()),
    }
}

macro_rules! transport_channel {
    (
        $(#[$open_meta:meta])*
        $vis:vis $name:ident ( $( $param:ident : $ty:ty ),* $(,)? ) -> $ret:ty,
            bus = $bus:ident,
            sink = $mk:ident,
            open = $open:block,
        $(#[$send_meta:meta])*
        send = $send:ident ( $msg:ty ) via $dispatch:path $(,)?
    ) => {
        $(#[$open_meta])*
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
                    $crate::transport::surface::ChannelSink::for_channel(channel.clone(), app.clone()),
                )
            };
            $open
        }

        $(#[$send_meta])*
        #[tauri::command]
        #[specta::specta]
        $vis async fn $send(
            bus: tauri::State<'_, $crate::transport::Bus>,
            key: ::std::string::String,
            msg: $msg,
        ) -> ::std::result::Result<(), ::std::string::String> {
            $dispatch(&bus, key, msg).await
        }
    };
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

async fn surface_channel_open(
    bus: &Bus,
    mint_sink: impl Fn() -> std::sync::Arc<dyn orchestrator::app::surface::SurfaceSink>,
    session_id: String,
    placement: String,
    cols: u16,
    rows: u16,
    cwd: Option<String>,
) -> Result<String, String> {
    if let Some(existing) = bus
        .query(FindSurfaceByPlacement {
            session: session_id.clone(),
            placement: placement.clone(),
        })
        .await
        .map_err(|e| e.to_string())?
    {
        bus.execute(SubscribeSurface {
            surface_id: existing.id.clone(),
            sink: mint_sink(),
        })
        .await
        .map_err(|e| e.to_string())?;
        let _ = bus
            .execute(DetachSurface {
                id: existing.id.clone(),
            })
            .await;
        match attach_surface(bus.cx(), &existing.id).await {
            Ok(()) => return Ok(existing.id),
            Err(_) => {
                let _ = bus
                    .execute(UnsubscribeSurface {
                        surface_id: existing.id.clone(),
                    })
                    .await;
                let _ = bus.execute(CloseSurface { id: existing.id }).await;
            }
        }
    }

    bus.execute(SpawnSurface {
        session: session_id.clone(),
        kind: "terminal".to_string(),
        cwd,
        placement: Some(placement.clone()),
        cols: Some(cols),
        rows: Some(rows),
    })
    .await
    .map_err(|e| e.to_string())?;

    let surface = bus
        .query(FindSurfaceByPlacement {
            session: session_id.clone(),
            placement,
        })
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "surface vanished after spawn".to_string())?;

    let id = surface.id;
    bus.execute(SubscribeSurface {
        surface_id: id.clone(),
        sink: mint_sink(),
    })
    .await
    .map_err(|e| e.to_string())?;
    let _ = attach_surface(bus.cx(), &id).await;

    let _ = bus
        .execute_notable(SurfaceStarted {
            surface_id: id.clone(),
            session_id: session_id.clone(),
            ts: now_ms(),
        })
        .await;
    Ok(id)
}

transport_channel! {
    pub surface_channel(
        session_id: String,
        placement: String,
        cols: u16,
        rows: u16,
        cwd: Option<String>,
    ) -> String,
    bus = bus,
    sink = mint_sink,
    open = {
        let id = surface_channel_open(&bus, &mint_sink, session_id, placement, cols, rows, cwd).await?;
        Ok(id)
    },
    send = surface_channel_send(SurfaceClientMsg) via route_surface_send,
}


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

transport_command!(surface_reconcile() => ReconcileSurfaces);

transport_create!(
    surface_spawn(session_id: String) -> String {
        let placement = uuid::Uuid::new_v4().to_string();
        execute: SpawnSurface {
            session: session_id.clone(),
            kind: "terminal".to_string(),
            cwd: None,
            placement: Some(placement.clone()),
            cols: None,
            rows: None,
        },
        read_back: FindSurfaceByPlacement {
            session: session_id,
            placement,
        },
        map: |surface| surface.id,
        missing: "surface vanished after spawn",
    }
);

transport_command!(surface_close(id: String) => CloseSurface { id });
transport_command!(surface_detach(id: String) => DetachSurface { id });

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use orchestrator::app::surface::{SubscribeSurface, SurfaceEvent, SurfaceSink};
    use orchestrator::boot::{test_ctx_with_probe, TestRuntimeProbe};
    use orchestrator::shared::Bus;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    use super::*;

    /// Counts the bus spans (`command`/`query`) opened in a closure -- the
    /// observable signal that a dispatch entered the telemetered pipeline.
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

    // An `Input` send reaches the runtime with the exact bytes, off the bus.
    #[tokio::test]
    async fn input_send_writes_the_bytes_to_the_runtime() {
        let (bus, probe) = bus_with_probe().await;

        route_surface_send(
            &bus,
            "sf_1".to_owned(),
            SurfaceClientMsg::Input {
                bytes: b"ls\n".to_vec(),
            },
        )
        .await
        .unwrap();

        assert_eq!(probe.inputs(), vec![("sf_1".to_owned(), b"ls\n".to_vec())]);
    }

    // An `Input` send opens no `command`/`query` span -- the payload never enters
    // the telemetered pipeline (keystroke-never-logged).
    #[tokio::test]
    async fn input_send_opens_no_bus_span() {
        let spans = BusSpans::default();
        let _guard = tracing_subscriber::registry()
            .with(spans.clone())
            .set_default();

        let (bus, _probe) = bus_with_probe().await;
        route_surface_send(
            &bus,
            "sf_1".to_owned(),
            SurfaceClientMsg::Input {
                bytes: b"secret".to_vec(),
            },
        )
        .await
        .unwrap();

        assert_eq!(*spans.0.lock().unwrap(), 0);
    }

    // A `Resize` send reaches the runtime, off the bus.
    #[tokio::test]
    async fn resize_send_reaches_the_runtime() {
        let (bus, probe) = bus_with_probe().await;

        route_surface_send(
            &bus,
            "sf_1".to_owned(),
            SurfaceClientMsg::Resize {
                cols: 120,
                rows: 40,
            },
        )
        .await
        .unwrap();

        assert_eq!(probe.resizes(), vec![("sf_1".to_owned(), 120, 40)]);
    }

    // Output frames produced by the surface source reach a registered sink without
    // further bus dispatch -- the registry delivers directly, per-frame.
    #[tokio::test]
    async fn output_frames_reach_the_registered_sink() {
        let (bus, _probe) = bus_with_probe().await;
        let seen: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
        let recorder = seen.clone();
        struct ByteRecorder(Arc<Mutex<Vec<Vec<u8>>>>);
        impl SurfaceSink for ByteRecorder {
            fn emit(&self, _surface: &str, event: &SurfaceEvent<'_>) {
                if let SurfaceEvent::Bytes(b) = event {
                    self.0.lock().unwrap().push(b.to_vec());
                }
            }
        }

        bus.execute(SubscribeSurface {
            surface_id: "sf_1".to_owned(),
            sink: Arc::new(ByteRecorder(recorder)),
        })
        .await
        .unwrap();

        bus.cx()
            .surface_sinks()
            .dispatch("sf_1", |s| s.emit("sf_1", &SurfaceEvent::Bytes(b"out")));

        assert_eq!(seen.lock().unwrap().as_slice(), [b"out".to_vec()]);
    }

    // Two `Input` sends arrive at the runtime in submission order.
    #[tokio::test]
    async fn two_inputs_arrive_in_order() {
        let (bus, probe) = bus_with_probe().await;

        route_surface_send(
            &bus,
            "sf_1".to_owned(),
            SurfaceClientMsg::Input {
                bytes: b"first".to_vec(),
            },
        )
        .await
        .unwrap();
        route_surface_send(
            &bus,
            "sf_1".to_owned(),
            SurfaceClientMsg::Input {
                bytes: b"second".to_vec(),
            },
        )
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
        impl SurfaceSink for Recorder {
            fn emit(&self, surface: &str, _event: &SurfaceEvent<'_>) {
                self.0.lock().unwrap().push(surface.to_owned());
            }
        }

        bus.execute(SubscribeSurface {
            surface_id: "sf_1".to_owned(),
            sink: Arc::new(Recorder(recorder)),
        })
        .await
        .unwrap();

        bus.cx()
            .surface_sinks()
            .dispatch("sf_1", |s| s.emit("sf_1", &SurfaceEvent::Bytes(b"pre")));
        assert_eq!(seen.lock().unwrap().as_slice(), ["sf_1".to_owned()]);

        route_surface_send(&bus, "sf_1".to_owned(), SurfaceClientMsg::Close)
            .await
            .unwrap();

        bus.cx()
            .surface_sinks()
            .dispatch("sf_1", |s| s.emit("sf_1", &SurfaceEvent::Bytes(b"post")));
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
        };
        assert_keys(
            &serde_json::to_value(s).unwrap(),
            &["id", "sessionId", "kind", "cwd", "status", "placement"],
        );
    }
}

//! Duplex channel transport. A client handle is two `#[tauri::command]` shims:
//! `open` registers a per-session receive `ChannelSink` over the renderer-provided
//! `ipc::Channel` (the bus-telemetered subscribe half), and `name_send` carries
//! every client->backend message as one tagged value. `Input`/`Resize` are written
//! straight to the runtime port off the bus -- no command object, no span, no
//! recording layer ever sees the payload (the keystroke-never-logged invariant);
//! only `Close` rides the bus (`UnsubscribeSurface`). `ipc::Channel` is send-only
//! and tauri has no duplex primitive, so two shims is the floor.

use orchestrator::app::surface::{resize_surface, send_surface_input, UnsubscribeSurface};
use serde::Deserialize;

use crate::transport::Bus;

/// A client->backend message on a surface channel. `Input` carries raw key bytes;
/// `Resize` carries the new geometry; `Close` tears the subscription down. The
/// tagged shape is the wire contract the renderer's `send`/`resize`/`close` map to.
#[derive(Debug, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SurfaceClientMsg {
    Input { bytes: Vec<u8> },
    Resize { cols: u16, rows: u16 },
    Close,
}

/// Route a surface channel send. `Input`/`Resize` write straight to the runtime
/// port on `bus.cx()` -- off the bus, so no span/log/recording layer observes the
/// payload. `Close` dispatches `UnsubscribeSurface` through the bus (telemetered).
pub async fn surface_channel_send(
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

/// Generate the two `#[tauri::command]` shims of a duplex channel endpoint.
///
/// `open`: `$name(channel, ...params) -> $ret`. Generic over the tauri runtime,
/// builds a per-channel `ChannelSink` via the `$mk` closure, and runs `$body` (which
/// `bus.execute`s the session-open command(s) that register the sink, returning the
/// session key).
///
/// `send`: `${name}_send(key, msg)`. Forwards to `$dispatch(bus, key, msg)` -- the
/// caller-provided off-telemetry router (`Input`/`Resize` -> runtime port; `Close`
/// -> `UnsubscribeSurface`). The message type and router are parameterized so the
/// macro is endpoint-agnostic.
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
                    $crate::transport::sink::ChannelSink::for_channel(channel.clone(), app.clone()),
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

pub(crate) use transport_channel;

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

        surface_channel_send(
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
        surface_channel_send(
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

        surface_channel_send(
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

        surface_channel_send(
            &bus,
            "sf_1".to_owned(),
            SurfaceClientMsg::Input {
                bytes: b"first".to_vec(),
            },
        )
        .await
        .unwrap();
        surface_channel_send(
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

    // A `Close` send dispatches `UnsubscribeSurface` through the bus, removing the
    // registered sink so a subsequent frame reaches no sink. Asserting the removed
    // sink (observable registry state) proves the unsubscribe deterministically,
    // without depending on tracing span creation (process-global `MAX_LEVEL`-gated
    // and therefore racy under parallel test threads).
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

        // Sanity: while subscribed, a dispatched frame reaches the sink.
        bus.cx()
            .surface_sinks()
            .dispatch("sf_1", |s| s.emit("sf_1", &SurfaceEvent::Bytes(b"pre")));
        assert_eq!(seen.lock().unwrap().as_slice(), ["sf_1".to_owned()]);

        surface_channel_send(&bus, "sf_1".to_owned(), SurfaceClientMsg::Close)
            .await
            .unwrap();

        // After Close, the sink is gone: a dispatched frame reaches nothing new.
        bus.cx()
            .surface_sinks()
            .dispatch("sf_1", |s| s.emit("sf_1", &SurfaceEvent::Bytes(b"post")));
        assert_eq!(seen.lock().unwrap().as_slice(), ["sf_1".to_owned()]);
    }
}

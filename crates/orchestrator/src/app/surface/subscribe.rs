//! The subscribe command. Host-constructed, never an invoke DTO: it carries an
//! `Arc<dyn SurfaceSink>` (not `Deserialize`) the transport shim builds, so the
//! wire-DTO conventions (`Deserialize`, primitive-only fields) do not apply. It
//! and its handler therefore live in an inner module, outside the file-scoped
//! position the message-dto rules match.

mod command {
    use std::sync::Arc;

    use crate::context::Ctx;
    use crate::events::surface::SurfaceSink;
    use crate::shared::message::Command;
    use crate::shared::Result;

    /// Register a client sink under a surface id. Dispatched through the bus
    /// once; afterwards the pump delivers every frame for the id straight to the
    /// sink, with no per-frame dispatch.
    pub struct SubscribeSurface {
        pub surface_id: String,
        pub sink: Arc<dyn SurfaceSink>,
    }

    impl Command<Ctx> for SubscribeSurface {
        async fn handle(&self, cx: &Ctx) -> Result<()> {
            cx.surface_sinks()
                .register(&self.surface_id, self.sink.clone());
            Ok(())
        }
    }

    /// Tear down a surface subscription: drop every sink registered under the
    /// id. Dispatched on close/detach; afterwards the pump finds no sink for the
    /// id and the stream goes quiet.
    pub struct UnsubscribeSurface {
        pub surface_id: String,
    }

    impl Command<Ctx> for UnsubscribeSurface {
        async fn handle(&self, cx: &Ctx) -> Result<()> {
            cx.surface_sinks().remove_key(&self.surface_id);
            Ok(())
        }
    }
}

pub use command::{SubscribeSurface, UnsubscribeSurface};

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::boot::test_ctx;
    use crate::events::surface::{SurfaceEvent, SurfaceSink};
    use crate::shared::bus::Bus;

    /// A sink that records the surface ids it is emitted for.
    struct Recorder(Arc<Mutex<Vec<String>>>);

    impl SurfaceSink for Recorder {
        fn emit(&self, surface: &str, _event: &SurfaceEvent<'_>) {
            self.0.lock().unwrap().push(surface.to_owned());
        }
    }

    /// Counts the bus spans (`command`/`query`) opened in a closure -- the
    /// observable signal that a dispatch entered the layered pipeline.
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

    #[tokio::test]
    async fn subscribing_dispatches_exactly_one_command_through_the_bus() {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        let spans = BusSpans::default();
        let _guard = tracing_subscriber::registry()
            .with(spans.clone())
            .set_default();

        let bus = Bus::new(test_ctx().await.unwrap());
        bus.execute(SubscribeSurface {
            surface_id: "sf_1".to_owned(),
            sink: Arc::new(Recorder(Arc::default())),
        })
        .await
        .unwrap();

        assert_eq!(*spans.0.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn after_subscribing_frames_reach_the_sink_with_no_further_dispatch() {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        let spans = BusSpans::default();
        let _guard = tracing_subscriber::registry()
            .with(spans.clone())
            .set_default();

        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let ctx = test_ctx().await.unwrap();
        let bus = Bus::new(ctx.clone());

        bus.execute(SubscribeSurface {
            surface_id: "sf_1".to_owned(),
            sink: Arc::new(Recorder(Arc::clone(&seen))),
        })
        .await
        .unwrap();

        // Emit several frames the way the pump does: straight through the
        // registry, no bus involved.
        for _ in 0..3 {
            ctx.surface_sinks()
                .dispatch("sf_1", |s| s.emit("sf_1", &SurfaceEvent::Bytes(b"x")));
        }

        assert_eq!(seen.lock().unwrap().as_slice(), ["sf_1", "sf_1", "sf_1"]);
        // Only the subscribe command dispatched -- never one-per-frame.
        assert_eq!(*spans.0.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn a_frame_for_another_surface_does_not_reach_the_sink() {
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let ctx = test_ctx().await.unwrap();
        let bus = Bus::new(ctx.clone());

        bus.execute(SubscribeSurface {
            surface_id: "sf_1".to_owned(),
            sink: Arc::new(Recorder(Arc::clone(&seen))),
        })
        .await
        .unwrap();

        ctx.surface_sinks()
            .dispatch("sf_2", |s| s.emit("sf_2", &SurfaceEvent::Bytes(b"x")));

        assert!(seen.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn after_unsubscribing_no_further_frames_reach_the_sink() {
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let ctx = test_ctx().await.unwrap();
        let bus = Bus::new(ctx.clone());

        bus.execute(SubscribeSurface {
            surface_id: "sf_1".to_owned(),
            sink: Arc::new(Recorder(Arc::clone(&seen))),
        })
        .await
        .unwrap();
        bus.execute(UnsubscribeSurface {
            surface_id: "sf_1".to_owned(),
        })
        .await
        .unwrap();

        ctx.surface_sinks()
            .dispatch("sf_1", |s| s.emit("sf_1", &SurfaceEvent::Bytes(b"x")));

        assert!(seen.lock().unwrap().is_empty());
    }
}

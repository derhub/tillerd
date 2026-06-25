use std::sync::Arc;

use crate::context::Ctx;
use crate::entities::SurfaceId;
use crate::infra::daemon_pty_api::{Output, Runtime};
use crate::shared::bus::Registry;
use crate::shared::domain_channel::{
    CloseDomainChannel, DomainChannelEvent, DomainChannelMessage, DomainChannelSink,
    DomainChannelStream, OpenDomainChannel,
};
use crate::shared::Result;

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct OpenSurfaceChannel {
    pub surface_id: String,
}

impl OpenDomainChannel<Ctx> for OpenSurfaceChannel {
    async fn handle(&self, cx: &Ctx, sink: Arc<dyn DomainChannelSink>) -> Result<()> {
        let id = SurfaceId::from_string(&self.surface_id);
        cx.domain_channel_sinks()
            .register(&format!("surface://{}", self.surface_id), sink);

        cx.runtime().attach(&id).await?;

        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CloseSurfaceChannel {
    pub surface_id: String,
}

impl CloseDomainChannel<Ctx> for CloseSurfaceChannel {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        cx.domain_channel_sinks()
            .remove_key(&format!("surface://{}", self.surface_id));

        cx.runtime()
            .detach(&SurfaceId::from_string(&self.surface_id))
            .await?;

        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SurfaceClientMsg {
    Input { bytes: Vec<u8> },
    Resize { cols: u16, rows: u16 },
}

impl DomainChannelMessage<Ctx> for SurfaceClientMsg {
    async fn handle(&self, cx: &Ctx, key: &str) -> Result<()> {
        let id = SurfaceId::from_string(key);
        match self {
            Self::Input { bytes } => cx.runtime().input(&id, bytes).await,
            Self::Resize { cols, rows } => cx.runtime().resize(&id, *cols, *rows).await,
        }
    }
}

pub struct SurfaceChannelStream {
    pub runtime: Arc<Runtime>,
    pub registry: Arc<Registry<dyn DomainChannelSink>>,
}

impl DomainChannelStream for SurfaceChannelStream {
    async fn handle(self) {
        while let Some(frame) = self.runtime.recv().await {
            let event = match &frame.output {
                Output::Bytes(b) => DomainChannelEvent::Bytes(b),
                Output::Status(s) => DomainChannelEvent::Status(s),
                Output::Exit(q) => DomainChannelEvent::Exit(q),
                Output::Error(r) => DomainChannelEvent::Error(r),
            };
            self.registry
                .dispatch(&format!("surface://{}", frame.surface), |s| s.emit(&event));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::surface::test_util::{harness, seed_session};
    use crate::infra::daemon_pty_api::RuntimeCall;
    use std::sync::Mutex;

    struct Recorder(Arc<Mutex<Vec<String>>>);
    impl DomainChannelSink for Recorder {
        fn emit(&self, event: &DomainChannelEvent<'_>) {
            if let DomainChannelEvent::Bytes(b) = event {
                self.0
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(b).into_owned());
            }
        }
    }

    #[tokio::test]
    async fn open_registers_sink_and_attaches_to_pty() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-channel").await;

        let surface = sqlx::query_as::<_, crate::app::surface::SurfaceView>(
            "INSERT INTO surface (id, session_id, kind, cwd, status, placement)
             VALUES ('sf_1', ?, 'terminal', '/work', 'live', 'main')
             RETURNING id, session_id, kind, cwd, status, placement",
        )
        .bind(&session)
        .fetch_one(&h.pool)
        .await
        .unwrap();

        let seen = Arc::new(Mutex::new(Vec::new()));
        let open_cmd = OpenSurfaceChannel {
            surface_id: surface.id.clone(),
        };

        open_cmd
            .handle(h.bus.cx(), Arc::new(Recorder(seen.clone())))
            .await
            .unwrap();

        let id = SurfaceId::from_string(&surface.id);
        assert_eq!(h.runtime.calls(), vec![RuntimeCall::Attach(id)]);

        h.bus
            .cx()
            .domain_channel_sinks()
            .dispatch(&format!("surface://{}", surface.id), |s| {
                s.emit(&DomainChannelEvent::Bytes(b"hello"))
            });

        assert_eq!(seen.lock().unwrap().as_slice(), &["hello".to_owned()]);
    }

    #[tokio::test]
    async fn close_unregisters_sink_and_detaches_from_pty() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-channel-close").await;

        let surface = sqlx::query_as::<_, crate::app::surface::SurfaceView>(
            "INSERT INTO surface (id, session_id, kind, cwd, status, placement)
             VALUES ('sf_2', ?, 'terminal', '/work', 'live', 'main')
             RETURNING id, session_id, kind, cwd, status, placement",
        )
        .bind(&session)
        .fetch_one(&h.pool)
        .await
        .unwrap();

        let seen = Arc::new(Mutex::new(Vec::new()));
        let open_cmd = OpenSurfaceChannel {
            surface_id: surface.id.clone(),
        };

        open_cmd
            .handle(h.bus.cx(), Arc::new(Recorder(seen.clone())))
            .await
            .unwrap();

        h.runtime.clear_calls();

        let close_cmd = CloseSurfaceChannel {
            surface_id: surface.id.clone(),
        };

        close_cmd.handle(h.bus.cx()).await.unwrap();

        let id = SurfaceId::from_string(&surface.id);
        assert_eq!(h.runtime.calls(), vec![RuntimeCall::Detach(id)]);

        h.bus
            .cx()
            .domain_channel_sinks()
            .dispatch(&format!("surface://{}", surface.id), |s| {
                s.emit(&DomainChannelEvent::Bytes(b"hello"))
            });

        assert!(seen.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn message_handles_input_and_resize() {
        let h = harness().await;
        let msg_input = SurfaceClientMsg::Input {
            bytes: b"keystroke".to_vec(),
        };

        msg_input.handle(h.bus.cx(), "sf_3").await.unwrap();

        let msg_resize = SurfaceClientMsg::Resize {
            cols: 120,
            rows: 40,
        };

        msg_resize.handle(h.bus.cx(), "sf_3").await.unwrap();

        let id = SurfaceId::from_string("sf_3");
        assert_eq!(
            h.runtime.calls(),
            vec![
                RuntimeCall::Input {
                    surface: id.clone(),
                    bytes: b"keystroke".to_vec(),
                },
                RuntimeCall::Resize {
                    surface: id,
                    cols: 120,
                    rows: 40,
                }
            ]
        );
    }
}

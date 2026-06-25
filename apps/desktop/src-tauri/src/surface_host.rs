//! Tauri bridge for surfaces. Bus commands (`spawn`/`close`/`detach`) persist and
//! coordinate through the managed `Bus<Ctx>`; the duplex `surface_channel` opens the
//! per-surface stream and `surface_channel_send_cmd` forwards client messages
//! (`input`/`resize` to the runtime port off the bus, `close` via the bus). Each
//! subscription registers a per-surface `ChannelSink` into the orchestrator's
//! surface-sink registry, keyed by surface id; teardown drops it via
//! `UnsubscribeSurface`.

use orchestrator::app::notification::SurfaceStarted;
use orchestrator::app::surface::{
    attach_surface, CloseSurface, DetachSurface, FindSurfaceByPlacement, SpawnSurface,
    SubscribeSurface, UnsubscribeSurface,
};
use orchestrator::shared::Bus;
use orchestrator::Ctx;
use tauri::State;

use crate::notification_host;
use crate::transport::channel::{surface_channel_send, transport_channel, SurfaceClientMsg};

transport_channel! {
    /// Open a surface duplex channel at a session + placement: revisit the existing
    /// surface (re-attach, replaying scrollback) or spawn a fresh one, register the
    /// renderer's receive sink, and return the surface id (the send key). Output
    /// frames flow over `channel`; client->backend messages go to
    /// `surface_channel_send`.
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
    /// Send a client->backend surface message: `Input`/`Resize` write straight to
    /// the runtime port (off telemetry), `Close` unsubscribes through the bus.
    send = surface_channel_send_cmd(SurfaceClientMsg) via surface_channel_send,
}

/// Open (or revisit) a surface at a session + placement, registering a fresh
/// per-surface sink each time. On revisit, the existing surface's proxy is
/// re-attached (replaying scrollback) rather than respawned. `mint_sink` is called
/// once per registered sink (a revisit-then-respawn fallthrough registers twice).
async fn surface_channel_open(
    bus: &Bus<Ctx>,
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
        // Drop any lingering proxy first so attach does a fresh subscribe
        // (replays scrollback) instead of an idempotent no-op.
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
            ts: notification_host::now_ms(),
        })
        .await;
    Ok(id)
}

/// Spawn a surface in a session with a minted placement. Returns the surface id.
#[tauri::command]
#[specta::specta]
pub async fn surface_spawn(bus: State<'_, Bus<Ctx>>, session_id: String) -> Result<String, String> {
    let placement = uuid::Uuid::new_v4().to_string();
    bus.execute(SpawnSurface {
        session: session_id.clone(),
        kind: "terminal".to_string(),
        cwd: None,
        placement: Some(placement.clone()),
        cols: None,
        rows: None,
    })
    .await
    .map_err(|e| e.to_string())?;
    let surface = bus
        .query(FindSurfaceByPlacement {
            session: session_id,
            placement,
        })
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "surface vanished after spawn".to_string())?;
    Ok(surface.id)
}

/// Close the surface bound to a session + placement: drop its subscription and
/// remove its runtime proxy + record.
#[tauri::command]
#[specta::specta]
pub async fn surface_close(
    bus: State<'_, Bus<Ctx>>,
    session_id: String,
    placement: String,
) -> Result<(), String> {
    let Some(surface) = bus
        .query(FindSurfaceByPlacement {
            session: session_id,
            placement,
        })
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    let _ = bus
        .execute(UnsubscribeSurface {
            surface_id: surface.id.clone(),
        })
        .await;
    bus.execute(CloseSurface { id: surface.id })
        .await
        .map_err(|e| e.to_string())
}

/// Detach a surface's proxy stream; the PTY keeps running in the daemon. Drops the
/// subscription so the stream goes quiet.
#[tauri::command]
#[specta::specta]
pub async fn surface_detach(bus: State<'_, Bus<Ctx>>, surface_id: String) -> Result<(), String> {
    let _ = bus
        .execute(UnsubscribeSurface {
            surface_id: surface_id.clone(),
        })
        .await;
    bus.execute(DetachSurface { id: surface_id })
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use orchestrator::app::surface::{SurfaceEvent, SurfaceSink};
    use orchestrator::boot::test_ctx_with_probe;

    use super::*;

    /// A sink that records the output bytes dispatched to it.
    struct ByteRecorder(Arc<Mutex<Vec<Vec<u8>>>>);

    impl SurfaceSink for ByteRecorder {
        fn emit(&self, _surface: &str, event: &SurfaceEvent<'_>) {
            if let SurfaceEvent::Bytes(b) = event {
                self.0.lock().unwrap().push(b.to_vec());
            }
        }
    }

    async fn bus_with_probe() -> (Bus<Ctx>, orchestrator::boot::TestRuntimeProbe) {
        let (cx, probe) = test_ctx_with_probe().await.unwrap();
        orchestrator::boot::seed_session(&cx, "s").await.unwrap();
        (Bus::new(cx), probe)
    }

    async fn open(bus: &Bus<Ctx>, seen: &Arc<Mutex<Vec<Vec<u8>>>>) -> Result<String, String> {
        let seen = seen.clone();
        surface_channel_open(
            bus,
            move || Arc::new(ByteRecorder(seen.clone())),
            "s".to_owned(),
            "p".to_owned(),
            80,
            24,
            None,
        )
        .await
    }

    // Opening a channel for a placement that already has a surface re-attaches the
    // existing surface instead of spawning a second one (revisit parity).
    #[tokio::test]
    async fn revisit_reattaches_without_a_second_spawn() {
        let (bus, probe) = bus_with_probe().await;
        let sink_a: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
        let sink_b: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();

        let first = open(&bus, &sink_a).await.unwrap();
        let second = open(&bus, &sink_b).await.unwrap();

        assert_eq!(first, second);
        assert_eq!(probe.spawns(), vec![first]);
    }

    // The revisit open attaches the existing surface's proxy -- the orchestrator
    // replays scrollback over that attach.
    #[tokio::test]
    async fn revisit_attaches_the_existing_surface() {
        let (bus, probe) = bus_with_probe().await;
        let sink_a: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
        let sink_b: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();

        let id = open(&bus, &sink_a).await.unwrap();
        open(&bus, &sink_b).await.unwrap();

        assert_eq!(probe.attaches(), vec![id.clone(), id]);
    }

    // The fresh sink registered on revisit receives surface output -- replayed
    // scrollback reaches the renderer that re-opened the channel.
    #[tokio::test]
    async fn revisit_sink_receives_replayed_output() {
        let (bus, _probe) = bus_with_probe().await;
        let sink_a: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
        let sink_b: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();

        let id = open(&bus, &sink_a).await.unwrap();
        open(&bus, &sink_b).await.unwrap();

        bus.cx()
            .surface_sinks()
            .dispatch(&id, |s| s.emit(&id, &SurfaceEvent::Bytes(b"scrollback")));

        assert_eq!(sink_b.lock().unwrap().as_slice(), [b"scrollback".to_vec()]);
    }
}

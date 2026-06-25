//! Tauri bridge for surfaces. Bus commands (`spawn`/`close`) persist and coordinate
//! through the managed `Bus<Ctx>`; the off-bus I/O channel (`input`/`resize`/
//! `attach`/`detach`) forwards to the runtime port. Each subscription registers a
//! per-surface `ChannelSink` into the orchestrator's surface-sink registry, keyed by
//! surface id; teardown drops it via `UnsubscribeSurface`. The wire -- command names
//! and argument shapes -- is unchanged.

use orchestrator::app::notification::SurfaceStarted;
use orchestrator::app::surface::{
    attach_surface, resize_surface, send_surface_input, CloseSurface, DetachSurface,
    FindSurfaceByPlacement, SpawnSurface, SubscribeSurface, UnsubscribeSurface,
};
use orchestrator::shared::Bus;
use orchestrator::Ctx;
use tauri::State;

use crate::notification_host;
use crate::transport::channel::{surface_channel_send, transport_channel, SurfaceClientMsg};
use crate::transport::macros::transport_subscribe;

transport_channel! {
    /// Open a surface duplex channel: spawn a surface at a session + placement,
    /// register the renderer's receive sink, and return the surface id (the send
    /// key). Output frames flow over `channel`; client->backend messages go to
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
                session: session_id,
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
        Ok(id)
    },
    /// Send a client->backend surface message: `Input`/`Resize` write straight to
    /// the runtime port (off telemetry), `Close` unsubscribes through the bus.
    send = surface_channel_send_cmd(SurfaceClientMsg) via surface_channel_send,
}

transport_subscribe! {
    /// Create (or revisit) a surface at a session + placement. On revisit, a fresh
    /// per-surface sink is registered and the proxy re-attached (replaying
    /// scrollback); otherwise a fresh surface is spawned. Returns the surface id.
    pub surface_create(
        session_id: String,
        placement: String,
        cols: u16,
        rows: u16,
        cwd: Option<String>,
    ) -> String,
    bus = bus,
    sink = mint_sink,
    {
        // Revisit: re-attach to the session's existing surface at this placement.
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

        // Spawn a fresh surface at this placement.
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

/// Send raw input bytes to a surface's PTY. Off the bus; the payload is never logged.
#[tauri::command]
#[specta::specta]
pub async fn surface_input(
    bus: State<'_, Bus<Ctx>>,
    surface_id: String,
    bytes: Vec<u8>,
) -> Result<(), String> {
    send_surface_input(bus.cx(), &surface_id, &bytes)
        .await
        .map_err(|e| e.to_string())
}

/// Resize a surface's PTY. Off the bus (high-frequency pass-through).
#[tauri::command]
#[specta::specta]
pub async fn surface_resize(
    bus: State<'_, Bus<Ctx>>,
    surface_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    resize_surface(bus.cx(), &surface_id, cols, rows)
        .await
        .map_err(|e| e.to_string())
}

/// Detach a surface's proxy stream; the PTY keeps running in the daemon. Drops the
/// subscription so the stream goes quiet.
#[tauri::command]
#[specta::specta]
pub async fn surface_detach(
    bus: State<'_, Bus<Ctx>>,
    surface_id: String,
) -> Result<(), String> {
    let _ = bus
        .execute(UnsubscribeSurface {
            surface_id: surface_id.clone(),
        })
        .await;
    bus.execute(DetachSurface { id: surface_id })
        .await
        .map_err(|e| e.to_string())
}

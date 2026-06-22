//! Tauri bridge for surfaces. Bus commands (`spawn`/`close`) persist and coordinate
//! through the managed `Bus<Ctx>`; the off-bus I/O channel (`input`/`resize`/
//! `attach`/`detach`) forwards to the runtime port. The per-surface output
//! `ipc::Channel` registry is shared with the runtime's `ChannelSink`. The wire --
//! command names and argument shapes -- is unchanged.

use orchestrator::app::surface::{
    attach_surface, resize_surface, send_surface_input, CloseSurface, DetachSurface,
    FindSurfaceByPlacement, SpawnSurface, SurfaceId,
};
use orchestrator::shared::Bus;
use orchestrator::Ctx;
use tauri::{AppHandle, State};

use crate::notification_host;
use crate::transport::sink::{register_channel, unregister_channel, SurfaceChannels};

/// Create (or revisit) a surface at a session + placement. On revisit, the existing
/// surface's output channel is re-registered and its proxy re-attached (replaying
/// scrollback); otherwise a fresh surface is spawned. Returns the surface id.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn surface_create<R: tauri::Runtime>(
    app: AppHandle<R>,
    bus: State<'_, Bus<Ctx>>,
    channels: State<'_, SurfaceChannels>,
    channel: tauri::ipc::Channel<Vec<u8>>,
    session_id: String,
    placement: String,
    cols: u16,
    rows: u16,
    cwd: Option<String>,
) -> Result<String, String> {
    // Revisit: re-attach to the session's existing surface at this placement.
    if let Some(existing) = bus
        .query(FindSurfaceByPlacement {
            session: session_id.clone(),
            placement: placement.clone(),
        })
        .await
        .map_err(|e| e.to_string())?
    {
        register_channel(&channels, &existing.id, channel.clone());
        // Drop any lingering proxy first so attach does a fresh subscribe (replays
        // scrollback) instead of an idempotent no-op.
        let _ = bus
            .execute(DetachSurface {
                id: existing.id.clone(),
            })
            .await;
        match attach_surface(bus.cx(), &SurfaceId::from_string(existing.id.clone())).await {
            Ok(()) => return Ok(existing.id),
            Err(_) => {
                unregister_channel(&channels, &existing.id);
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
    register_channel(&channels, &id, channel);
    let _ = attach_surface(bus.cx(), &SurfaceId::from_string(id.clone())).await;

    notification_host::record(
        &app,
        &bus,
        notification_host::surface_started(&id, &session_id, notification_host::now_ms()),
    )
    .await;
    Ok(id)
}

/// Spawn a surface in a session with a minted placement. Returns the surface id.
#[tauri::command]
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

/// Close the surface bound to a session + placement: drop its output channel and
/// remove its runtime proxy + record.
#[tauri::command]
pub async fn surface_close(
    bus: State<'_, Bus<Ctx>>,
    channels: State<'_, SurfaceChannels>,
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
    unregister_channel(&channels, &surface.id);
    bus.execute(CloseSurface { id: surface.id })
        .await
        .map_err(|e| e.to_string())
}

/// Send raw input bytes to a surface's PTY. Off the bus; the payload is never logged.
#[tauri::command]
pub async fn surface_input(
    bus: State<'_, Bus<Ctx>>,
    surface_id: String,
    bytes: Vec<u8>,
) -> Result<(), String> {
    send_surface_input(bus.cx(), &SurfaceId::from_string(surface_id), &bytes)
        .await
        .map_err(|e| e.to_string())
}

/// Resize a surface's PTY. Off the bus (high-frequency pass-through).
#[tauri::command]
pub async fn surface_resize(
    bus: State<'_, Bus<Ctx>>,
    surface_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    resize_surface(bus.cx(), &SurfaceId::from_string(surface_id), cols, rows)
        .await
        .map_err(|e| e.to_string())
}

/// Detach a surface's proxy stream; the PTY keeps running in the daemon. Drops the
/// output channel so the stream goes quiet.
#[tauri::command]
pub async fn surface_detach(
    bus: State<'_, Bus<Ctx>>,
    channels: State<'_, SurfaceChannels>,
    surface_id: String,
) -> Result<(), String> {
    unregister_channel(&channels, &surface_id);
    bus.execute(DetachSurface { id: surface_id })
        .await
        .map_err(|e| e.to_string())
}

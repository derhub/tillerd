use crate::context::Ctx;
use crate::entities::{Surface, SurfaceId};
use crate::infra::runtime::Geometry;
use crate::infra::SurfaceRepo;
use crate::shared::{Error, Result};

/// Terminal dimensions a spawned surface starts at; the renderer resizes on attach.
pub(super) const DEFAULT_GEOMETRY: Geometry = Geometry { cols: 80, rows: 24 };

pub(super) fn default_cwd() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/".to_owned())
}

pub(super) async fn require_surface(cx: &Ctx, id: &SurfaceId) -> Result<Surface> {
    SurfaceRepo::get(cx.db(), id)
        .await?
        .ok_or_else(|| Error::SurfaceNotFound(id.as_str().to_owned()))
}

/// Every surface as a typed entity, oldest first. Command path: `ReconcileSurfaces`
/// reads `kind`/`cwd`/`id` off each row to rebuild a spawn request, so this stays
/// the write-model entity, not the `*View` read DTO.
pub(super) async fn all_surfaces(cx: &Ctx) -> Result<Vec<Surface>> {
    Ok(sqlx::query_as::<_, Surface>(
        "SELECT id, session_id, kind, cwd, status, placement
         FROM surface ORDER BY created_at ASC",
    )
    .fetch_all(cx.db())
    .await?)
}

// -- Off-bus surface I/O channel (never logged, no command object) ---------------

/// Send raw input bytes to a surface's PTY. Off the bus: no command object, no span,
/// no telemetry -- the payload must never reach a log.
pub async fn send_surface_input(cx: &Ctx, id: &str, bytes: &[u8]) -> Result<()> {
    cx.runtime().input(&SurfaceId::from_string(id), bytes).await
}

/// Resize a surface's PTY. Off the bus (high-frequency pass-through).
pub async fn resize_surface(cx: &Ctx, id: &str, cols: u16, rows: u16) -> Result<()> {
    cx.runtime()
        .resize(&SurfaceId::from_string(id), cols, rows)
        .await
}

/// Connect the proxy stream to an already-running daemon PTY. Lazy, per surface,
/// driven by the renderer registering its Channel -- there is no eager boot
/// attach-all. Off the bus.
pub async fn attach_surface(cx: &Ctx, id: &str) -> Result<()> {
    cx.runtime().attach(&SurfaceId::from_string(id)).await
}

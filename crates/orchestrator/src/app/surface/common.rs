use sqlx::Row;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::entities::{Surface, SurfaceId, SurfaceKind, SurfaceStatus};
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

pub(super) async fn all_surfaces(cx: &Ctx) -> Result<Vec<Surface>> {
    let rows = sqlx::query(
        "SELECT id, session_id, kind, cwd, placement, status FROM surface ORDER BY created_at ASC",
    )
    .fetch_all(cx.db())
    .await?;
    rows.into_iter().map(row_to_surface).collect()
}

pub(super) fn row_to_surface(row: sqlx::sqlite::SqliteRow) -> Result<Surface> {
    let kind = match row.try_get::<String, _>("kind")?.as_str() {
        "terminal" => SurfaceKind::Terminal,
        "diff" => SurfaceKind::Diff,
        other => {
            return Err(Error::Validation {
                field: "kind",
                reason: format!("unknown surface kind: {other}"),
            })
        }
    };
    let status = match row.try_get::<String, _>("status")?.as_str() {
        "pending" => SurfaceStatus::Pending,
        "live" => SurfaceStatus::Live,
        "idle" => SurfaceStatus::Idle,
        "failed" => SurfaceStatus::Failed,
        other => {
            return Err(Error::Validation {
                field: "status",
                reason: format!("unknown surface status: {other}"),
            })
        }
    };
    Ok(Surface {
        id: SurfaceId::from_string(row.try_get::<String, _>("id")?),
        session_id: SessionId::from_string(row.try_get::<String, _>("session_id")?),
        kind,
        cwd: row.try_get("cwd")?,
        status,
        placement: row.try_get("placement")?,
    })
}

// ── Off-bus surface I/O channel (never logged, no command object) ───────────────

/// Send raw input bytes to a surface's PTY. Off the bus: no command object, no span,
/// no telemetry — the payload must never reach a log.
pub async fn send_surface_input(cx: &Ctx, id: &SurfaceId, bytes: &[u8]) -> Result<()> {
    cx.runtime().input(id, bytes).await
}

/// Resize a surface's PTY. Off the bus (high-frequency pass-through).
pub async fn resize_surface(cx: &Ctx, id: &SurfaceId, cols: u16, rows: u16) -> Result<()> {
    cx.runtime().resize(id, cols, rows).await
}

/// Connect the proxy stream to an already-running daemon PTY. Lazy, per surface,
/// driven by the renderer registering its Channel — there is no eager boot
/// attach-all. Off the bus.
pub async fn attach_surface(cx: &Ctx, id: &SurfaceId) -> Result<()> {
    cx.runtime().attach(id).await
}

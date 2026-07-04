use crate::context::Ctx;
use crate::entities::{Surface, SurfaceId};
use crate::infra::daemon_pty_api::Geometry;
use crate::infra::SurfaceRepo;
use crate::shared::{Error, Result};

pub(super) const DEFAULT_GEOMETRY: Geometry = Geometry { cols: 80, rows: 24 };

pub(super) fn default_cwd() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/".to_owned())
}

/// Wall-clock millis for stamping `surface.spawned_at` at the moment the PTY
/// is confirmed running (elapsed-since-spawn display, ui-panel-compound spec).
pub(super) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(super) async fn require_surface(cx: &Ctx, id: &SurfaceId) -> Result<Surface> {
    SurfaceRepo::get(cx.db(), id)
        .await?
        .ok_or_else(|| Error::SurfaceNotFound(id.as_str().to_owned()))
}

pub(super) async fn all_surfaces(cx: &Ctx) -> Result<Vec<Surface>> {
    Ok(sqlx::query_as::<_, Surface>(
        "SELECT id, session_id, kind, cwd, status, placement
         FROM surface ORDER BY created_at ASC",
    )
    .fetch_all(cx.db())
    .await?)
}

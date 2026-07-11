use serde::Deserialize;

use crate::app::surface::SurfaceView;
use crate::context::Ctx;
use crate::shared::errors::Result;
use crate::shared::message::Query;

/// One surface by id.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSurfaceById {
    pub id: String,
}

impl Query<Ctx> for GetSurfaceById {
    type Out = Option<SurfaceView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        Ok(sqlx::query_as::<_, SurfaceView>(
            "SELECT id, session_id, kind, cwd, status, placement, spawned_at
             FROM surface WHERE id = ?",
        )
        .bind(&self.id)
        .fetch_optional(cx.db())
        .await?)
    }
}

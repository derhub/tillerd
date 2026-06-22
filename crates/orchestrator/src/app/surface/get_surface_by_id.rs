use crate::context::Ctx;
use crate::entities::{Surface, SurfaceId};
use crate::infra::SurfaceRepo;
use crate::shared::cqs::Query;
use crate::shared::errors::Result;

/// One surface by id.
#[derive(Debug, Clone)]
pub struct GetSurfaceById {
    pub id: SurfaceId,
}

impl Query<Ctx> for GetSurfaceById {
    type Out = Option<Surface>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        SurfaceRepo::get(cx.db(), &self.id).await
    }
}

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::entities::Surface;
use crate::infra::SurfaceRepo;
use crate::shared::cqs::Query;
use crate::shared::errors::Result;
use crate::shared::pagination::{Listing, Page};

/// A session's surfaces, live-first.
#[derive(Debug, Clone)]
pub struct ListSurfacesBySession {
    pub session: SessionId,
    pub page: Page,
}

impl Query<Ctx> for ListSurfacesBySession {
    type Out = Listing<Surface>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        SurfaceRepo::list(cx.db(), &self.session, self.page.clone()).await
    }
}

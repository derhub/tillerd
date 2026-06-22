use crate::context::Ctx;
use crate::entities::session::Session;
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Query;
use crate::shared::errors::Result;
use crate::shared::pagination::{Listing, Page};

/// List sessions in a project, pinned-first.
pub struct ListSessionsByProject {
    pub project_id: crate::entities::project::ProjectId,
    pub page: Page,
}

impl Query<Ctx> for ListSessionsByProject {
    type Out = Listing<Session>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        SessionRepo::list(cx.db(), &self.project_id, self.page.clone()).await
    }
}

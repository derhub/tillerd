use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Query;
use crate::shared::errors::{Error, Result};

/// Return the session's panel-tree geometry (independent of the recipe).
pub struct GetPanelTree {
    pub id: SessionId,
}

impl Query<Ctx> for GetPanelTree {
    type Out = Option<String>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        SessionRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.as_str().to_owned()))?;
        SessionRepo::get_panel_tree(cx.db(), &self.id).await
    }
}

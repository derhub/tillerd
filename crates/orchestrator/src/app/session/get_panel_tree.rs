use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Query;

/// Return the session's panel-tree geometry (independent of the recipe).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPanelTree {
    pub id: String,
}

impl Query<Ctx> for GetPanelTree {
    type Out = Option<String>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let id = SessionId::from_string(&self.id);
        SessionRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.clone()))?;
        SessionRepo::get_panel_tree(cx.db(), &id).await
    }
}

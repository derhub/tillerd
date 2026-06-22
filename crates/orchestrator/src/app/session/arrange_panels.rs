use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Command;
use crate::shared::errors::{Error, Result};

/// Set the session's panel-tree geometry (how placements split into panes/tabs).
/// Independent of the launch spec.
pub struct ArrangePanels {
    pub id: SessionId,
    pub panel_tree_json: String,
}

impl Command<Ctx> for ArrangePanels {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        SessionRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.as_str().to_owned()))?;
        SessionRepo::set_panel_tree(cx.db(), &self.id, &self.panel_tree_json).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::get_panel_tree::GetPanelTree;
    use crate::app::session::test_util::{create_one, ctx};

    #[tokio::test]
    async fn get_panel_tree_returns_none_before_arrange() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;
        let tree = bus.query(GetPanelTree { id }).await.unwrap();
        assert!(tree.is_none());
    }

    #[tokio::test]
    async fn arrange_panels_persists_and_get_panel_tree_reads_it() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;

        bus.execute(ArrangePanels {
            id: id.clone(),
            panel_tree_json: r#"{"split":"h","ratio":0.5}"#.to_owned(),
        })
        .await
        .unwrap();

        let tree = bus.query(GetPanelTree { id }).await.unwrap();
        assert_eq!(tree.as_deref(), Some(r#"{"split":"h","ratio":0.5}"#));
    }
}

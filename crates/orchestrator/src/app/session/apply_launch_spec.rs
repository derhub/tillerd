use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Command;

/// Replace the session's launch spec (the recipe plane: which surfaces + placements).
/// Does not alter the panel-tree geometry.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyLaunchSpec {
    pub id: String,
    pub spec_version: u32,
    pub spec_json: String,
}

impl Command<Ctx> for ApplyLaunchSpec {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = SessionId::from_string(&self.id);
        let mut s = SessionRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.clone()))?;
        s.spec_version = Some(self.spec_version);
        s.spec_json = Some(self.spec_json.clone());
        SessionRepo::update(cx.db(), &s).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::arrange_panels::ArrangePanels;
    use crate::app::session::get_launch_spec::GetLaunchSpec;
    use crate::app::session::get_panel_tree::GetPanelTree;
    use crate::app::session::test_util::{create_one, ctx};

    // Scenario: Recipe and geometry are independent planes
    #[tokio::test]
    async fn apply_launch_spec_does_not_alter_panel_tree() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;

        bus.execute(ArrangePanels {
            id: id.clone(),
            panel_tree_json: r#"{"split":"h"}"#.to_owned(),
        })
        .await
        .unwrap();

        bus.execute(ApplyLaunchSpec {
            id: id.clone(),
            spec_version: 1,
            spec_json: r#"{"version":1,"items":[]}"#.to_owned(),
        })
        .await
        .unwrap();

        let tree = bus.query(GetPanelTree { id }).await.unwrap();
        assert_eq!(tree.as_deref(), Some(r#"{"split":"h"}"#));
    }

    #[tokio::test]
    async fn arrange_panels_does_not_alter_launch_spec() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;

        bus.execute(ApplyLaunchSpec {
            id: id.clone(),
            spec_version: 1,
            spec_json: r#"{"version":1,"items":[]}"#.to_owned(),
        })
        .await
        .unwrap();

        bus.execute(ArrangePanels {
            id: id.clone(),
            panel_tree_json: r#"{"split":"v"}"#.to_owned(),
        })
        .await
        .unwrap();

        let spec = bus.query(GetLaunchSpec { id }).await.unwrap();
        assert!(spec.is_some());
    }
}

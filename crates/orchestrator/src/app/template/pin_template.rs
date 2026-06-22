use crate::context::Ctx;
use crate::entities::template::TemplateId;
use crate::shared::{cqs::Command, Result};

use super::common::set_pinned;

/// Pin a library template (pinned items sort first in `ListTemplates`).
pub struct PinTemplate {
    pub id: TemplateId,
}

impl Command<Ctx> for PinTemplate {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        set_pinned(cx, &self.id, true).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;

    use super::super::import_template::ImportTemplate;
    use super::super::list_templates::ListTemplates;

    #[tokio::test]
    async fn pinned_template_sorts_before_unpinned() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

        bus.execute(ImportTemplate {
            name: "alpha".to_owned(),
            spec_version: 1,
            spec_json: "{}".to_owned(),
        })
        .await
        .unwrap();
        bus.execute(ImportTemplate {
            name: "beta".to_owned(),
            spec_version: 1,
            spec_json: "{}".to_owned(),
        })
        .await
        .unwrap();

        let before = bus.query(ListTemplates).await.unwrap();
        let beta_id = before
            .iter()
            .find(|t| t.name == "beta")
            .unwrap()
            .id
            .clone();

        bus.execute(PinTemplate { id: beta_id }).await.unwrap();

        let after = bus.query(ListTemplates).await.unwrap();
        assert_eq!(after[0].name, "beta", "pinned beta must sort first");
        assert_eq!(after[1].name, "alpha");
    }
}

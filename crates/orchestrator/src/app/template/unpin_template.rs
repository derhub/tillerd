use crate::context::Ctx;
use crate::entities::template::TemplateId;
use crate::shared::{cqs::Command, Result};

use super::common::set_pinned;

/// Unpin a library template.
pub struct UnpinTemplate {
    pub id: TemplateId,
}

impl Command<Ctx> for UnpinTemplate {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        set_pinned(cx, &self.id, false).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;

    use super::super::import_template::ImportTemplate;
    use super::super::list_templates::ListTemplates;
    use super::super::pin_template::PinTemplate;

    #[tokio::test]
    async fn unpin_template_restores_insertion_order() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

        bus.execute(ImportTemplate {
            name: "a".to_owned(),
            spec_version: 1,
            spec_json: "{}".to_owned(),
        })
        .await
        .unwrap();
        bus.execute(ImportTemplate {
            name: "b".to_owned(),
            spec_version: 1,
            spec_json: "{}".to_owned(),
        })
        .await
        .unwrap();

        let all = bus.query(ListTemplates).await.unwrap();
        let b_id = all.iter().find(|t| t.name == "b").unwrap().id.clone();

        bus.execute(PinTemplate { id: b_id.clone() }).await.unwrap();
        let pinned = bus.query(ListTemplates).await.unwrap();
        assert_eq!(pinned[0].name, "b");

        bus.execute(UnpinTemplate { id: b_id }).await.unwrap();
        let unpinned = bus.query(ListTemplates).await.unwrap();
        assert_eq!(unpinned[0].name, "a");
    }
}

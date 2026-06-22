use crate::context::Ctx;
use crate::entities::template::{Template, TemplateId};
use crate::shared::{cqs::Query, Result};

use super::common::{load_template, TemplateIndex};

/// Fetch one library template by id. Returns `None` if absent.
pub struct GetTemplateById {
    pub id: TemplateId,
}

impl Query<Ctx> for GetTemplateById {
    type Out = Option<Template>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let index = TemplateIndex::load(cx.fs_root()).await?;
        match index.entries.iter().find(|e| e.id == self.id.as_str()) {
            None => Ok(None),
            Some(entry) => Ok(Some(load_template(cx.fs_root(), entry).await?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;

    use super::super::import_template::ImportTemplate;
    use super::super::list_templates::ListTemplates;

    #[tokio::test]
    async fn get_template_by_id_returns_none_for_absent_id() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

        let got = bus
            .query(GetTemplateById {
                id: TemplateId::mint(),
            })
            .await
            .unwrap();

        assert!(got.is_none());
    }

    #[tokio::test]
    async fn get_template_by_id_returns_the_imported_template() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

        bus.execute(ImportTemplate {
            name: "bundle-a".to_owned(),
            spec_version: 1,
            spec_json: r#"{"items":["x"]}"#.to_owned(),
        })
        .await
        .unwrap();

        let all = bus.query(ListTemplates).await.unwrap();
        let id = all[0].id.clone();

        let got = bus
            .query(GetTemplateById { id: id.clone() })
            .await
            .unwrap()
            .unwrap();

        assert_eq!(got.id, id);
        assert_eq!(got.spec_json, r#"{"items":["x"]}"#);
    }
}

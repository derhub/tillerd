use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::template::TemplateId;
use crate::shared::{self, message::Command, Error, Result};

use super::common::{template_bundle_path, TemplateIndex};

/// Remove a custom template from the library. Rejects Prebuilt templates.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardTemplate {
    pub id: String,
}

impl Command<Ctx> for DiscardTemplate {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut index = TemplateIndex::load(cx.fs_root()).await?;
        let pos = index
            .entries
            .iter()
            .position(|e| e.id == self.id)
            .ok_or_else(|| Error::TemplateNotFound(self.id.clone()))?;

        if index.entries[pos].origin == "prebuilt" {
            return Err(Error::PrebuiltImmutable { kind: "template" });
        }

        let entry = index.entries.remove(pos);
        index.save(cx.fs_root()).await?;

        let bundle_path = template_bundle_path(cx.fs_root(), &TemplateId::from_string(&entry.id));
        shared::fs::delete(&bundle_path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;

    use super::super::common::{index_path, template_bundle_path, IndexEntry, TemplateIndex};
    use super::super::get_template_by_id::GetTemplateById;
    use super::super::import_template::ImportTemplate;
    use super::super::list_templates::ListTemplates;

    #[tokio::test]
    async fn discard_template_rejects_prebuilt() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

        // Manually inject a prebuilt entry.
        let id = TemplateId::mint();
        let bundle_path = template_bundle_path(dir.path(), &id);
        std::fs::create_dir_all(bundle_path.parent().unwrap()).unwrap();
        std::fs::write(&bundle_path, r#"{"items":[]}"#).unwrap();
        let index = TemplateIndex {
            entries: vec![IndexEntry {
                id: id.as_str().to_owned(),
                name: "prebuilt-tmpl".to_owned(),
                origin: "prebuilt".to_owned(),
                pinned: false,
                spec_version: 1,
            }],
        };
        let s = serde_json::to_string_pretty(&index).unwrap();
        std::fs::write(index_path(dir.path()), s).unwrap();

        let err = bus
            .execute(DiscardTemplate {
                id: id.as_str().to_owned(),
            })
            .await
            .unwrap_err();

        assert_eq!(err.code(), "prebuilt.immutable");
    }

    #[tokio::test]
    async fn discard_template_removes_a_custom_template() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

        bus.execute(ImportTemplate {
            name: "removable".to_owned(),
            spec_version: 1,
            spec_json: "{}".to_owned(),
        })
        .await
        .unwrap();

        let all = bus.query(ListTemplates).await.unwrap();
        let id = all[0].id.clone();

        bus.execute(DiscardTemplate { id: id.clone() })
            .await
            .unwrap();

        let remaining = bus.query(ListTemplates).await.unwrap();
        assert!(remaining.is_empty());

        let got = bus.query(GetTemplateById { id }).await.unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn discard_template_on_missing_id_returns_not_found() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

        let err = bus
            .execute(DiscardTemplate {
                id: TemplateId::mint().as_str().to_owned(),
            })
            .await
            .unwrap_err();

        assert_eq!(err.code(), "template.not_found");
    }
}

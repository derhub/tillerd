use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::template::TemplateId;
use crate::shared::{self, message::Command, Result};

use super::common::{template_bundle_path, IndexEntry, TemplateIndex};

/// Import a custom template bundle into the library.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTemplate {
    pub name: String,
    pub spec_version: u32,
    pub spec_json: String,
}

impl Command<Ctx> for ImportTemplate {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = TemplateId::mint();
        let bundle_path = template_bundle_path(cx.fs_root(), &id);
        if let Some(parent) = bundle_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        shared::fs::write_string(&bundle_path, &self.spec_json).await?;

        let mut index = TemplateIndex::load(cx.fs_root()).await?;
        index.entries.push(IndexEntry {
            id: id.as_str().to_owned(),
            name: self.name.clone(),
            origin: "custom".to_owned(),
            pinned: false,
            spec_version: self.spec_version,
        });
        index.save(cx.fs_root()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;

    use super::super::list_templates::ListTemplates;

    #[tokio::test]
    async fn import_template_adds_it_to_the_library() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

        bus.execute(ImportTemplate {
            name: "my-bundle".to_owned(),
            spec_version: 1,
            spec_json: r#"{"items":[]}"#.to_owned(),
        })
        .await
        .unwrap();

        let templates = bus.query(ListTemplates).await.unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "my-bundle");
        assert_eq!(templates[0].origin, "custom");
    }
}

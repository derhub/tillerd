use std::path::PathBuf;

use crate::context::Ctx;
use crate::entities::template::TemplateId;
use crate::shared::{self, cqs::Command, Error, Result};

use super::common::{template_bundle_path, TemplateIndex};

/// Write a copy of a template bundle to an export destination path.
pub struct ExportTemplate {
    pub id: TemplateId,
    pub dest_path: PathBuf,
}

impl Command<Ctx> for ExportTemplate {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let index = TemplateIndex::load(cx.fs_root()).await?;
        let entry = index
            .entries
            .iter()
            .find(|e| e.id == self.id.as_str())
            .ok_or_else(|| Error::TemplateNotFound(self.id.as_str().to_owned()))?;
        let src = template_bundle_path(cx.fs_root(), &TemplateId::from_string(&entry.id));
        let spec_json = shared::fs::read_string(&src).await?;
        shared::fs::write_string(&self.dest_path, &spec_json).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;

    use super::super::import_template::ImportTemplate;
    use super::super::list_templates::ListTemplates;

    #[tokio::test]
    async fn export_template_writes_spec_json_to_dest() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;
        let dest = dir.path().join("exported.json");

        bus.execute(ImportTemplate {
            name: "exportable".to_owned(),
            spec_version: 1,
            spec_json: r#"{"items":["export-me"]}"#.to_owned(),
        })
        .await
        .unwrap();

        let all = bus.query(ListTemplates).await.unwrap();
        let id = all[0].id.clone();

        bus.execute(ExportTemplate {
            id,
            dest_path: dest.clone(),
        })
        .await
        .unwrap();

        let content = std::fs::read_to_string(&dest).unwrap();
        assert_eq!(content, r#"{"items":["export-me"]}"#);
    }

    #[tokio::test]
    async fn export_template_on_missing_id_returns_not_found() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;
        let dest = dir.path().join("nope.json");

        let err = bus
            .execute(ExportTemplate {
                id: TemplateId::mint(),
                dest_path: dest,
            })
            .await
            .unwrap_err();

        assert_eq!(err.code(), "template.not_found");
    }
}

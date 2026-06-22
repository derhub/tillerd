use crate::context::Ctx;
use crate::entities::template::Template;
use crate::shared::{cqs::Query, Result};

use super::common::{load_template, TemplateIndex};

/// List the template library (prebuilt + custom), pinned-first.
pub struct ListTemplates;

impl Query<Ctx> for ListTemplates {
    type Out = Vec<Template>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let index = TemplateIndex::load(cx.fs_root()).await?;
        let mut entries = index.entries.clone();
        // stable sort: pinned DESC, then original insertion order
        entries.sort_by(|a, b| b.pinned.cmp(&a.pinned));
        let mut templates = Vec::with_capacity(entries.len());
        for entry in &entries {
            templates.push(load_template(cx.fs_root(), entry).await?);
        }
        Ok(templates)
    }
}

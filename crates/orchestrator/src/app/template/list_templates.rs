use serde::Deserialize;

use crate::app::template::TemplateView;
use crate::context::Ctx;
use crate::shared::{message::Query, Result};

use super::common::{load_template_view, TemplateIndex};

/// List the template library (prebuilt + custom), pinned-first.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTemplates;

impl Query<Ctx> for ListTemplates {
    type Out = Vec<TemplateView>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let index = TemplateIndex::load(cx.fs_root()).await?;
        let mut entries = index.entries.clone();
        // stable sort: pinned DESC, then original insertion order
        entries.sort_by(|a, b| b.pinned.cmp(&a.pinned));
        let mut templates = Vec::with_capacity(entries.len());
        for entry in &entries {
            templates.push(load_template_view(cx.fs_root(), entry).await?);
        }
        Ok(templates)
    }
}

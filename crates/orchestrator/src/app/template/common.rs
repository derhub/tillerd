use std::path::PathBuf;

use crate::app::template::view::TemplateView;
use crate::entities::template::TemplateId;
use crate::shared::{self, Error, Result};

pub(super) fn template_bundle_path(fs_root: &std::path::Path, id: &TemplateId) -> PathBuf {
    fs_root
        .join("templates")
        .join(format!("{}.json", id.as_str()))
}

pub(super) fn index_path(fs_root: &std::path::Path) -> PathBuf {
    fs_root.join("templates").join("index.json")
}

// -- index serialisation (serde only used inside this module, not on CQS structs) --

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub(super) struct TemplateIndex {
    pub(super) entries: Vec<IndexEntry>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(super) struct IndexEntry {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) origin: String,
    pub(super) pinned: bool,
    pub(super) spec_version: u32,
}

impl TemplateIndex {
    pub(super) async fn load(fs_root: &std::path::Path) -> Result<Self> {
        let path = index_path(fs_root);
        match shared::fs::read_string(&path).await {
            Ok(s) => Ok(serde_json::from_str(&s)?),
            Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(TemplateIndex::default())
            }
            Err(e) => Err(e),
        }
    }

    pub(super) async fn save(&self, fs_root: &std::path::Path) -> Result<()> {
        let path = index_path(fs_root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = serde_json::to_string_pretty(self)?;
        shared::fs::write_string(&path, &s).await
    }
}

/// Assemble a flat [`TemplateView`] from an index entry + its on-disk bundle.
///
/// Returns the serialisable read model rather than the domain entity (queries
/// return Views, not entities).
pub(super) async fn load_template_view(
    fs_root: &std::path::Path,
    entry: &IndexEntry,
) -> Result<TemplateView> {
    let id = TemplateId::from_string(&entry.id);
    let path = template_bundle_path(fs_root, &id);
    let spec_json = shared::fs::read_string(&path).await?;
    Ok(TemplateView {
        id: entry.id.clone(),
        name: entry.name.clone(),
        origin: entry.origin.clone(),
        pinned: entry.pinned,
        spec_version: entry.spec_version,
        spec_json,
    })
}

pub(super) async fn set_pinned(
    cx: &crate::context::Ctx,
    id: &TemplateId,
    pinned: bool,
) -> Result<()> {
    let mut index = TemplateIndex::load(cx.fs_root()).await?;
    let entry = index
        .entries
        .iter_mut()
        .find(|e| e.id == id.as_str())
        .ok_or_else(|| Error::TemplateNotFound(id.as_str().to_owned()))?;
    entry.pinned = pinned;
    index.save(cx.fs_root()).await
}

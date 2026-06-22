//! Portable template library entity: a named bundle (prebuilt or custom) selectable
//! at session creation. Prebuilt templates are immutable; custom ones can be discarded.
//! Stored as file bundles via `shared::fs`.

/// Stable identifier for a library template.
#[derive(Debug, Clone, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(transparent)]
pub struct TemplateId(String);

impl TemplateId {
    pub fn mint() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

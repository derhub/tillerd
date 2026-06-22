//! Portable template library entity: a named bundle (prebuilt or custom) selectable
//! at session creation. Prebuilt templates are immutable; custom ones can be discarded.
//! Stored as file bundles via `shared::fs`.

use crate::shared::{Error, Result};

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

/// Origin discriminator shared with the command library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum TemplateOrigin {
    Prebuilt,
    Custom,
}

impl TemplateOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            TemplateOrigin::Prebuilt => "prebuilt",
            TemplateOrigin::Custom => "custom",
        }
    }
}

/// A portable template bundle from the library.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Template {
    pub id: TemplateId,
    pub name: String,
    pub origin: TemplateOrigin,
    pub pinned: bool,
    pub spec_version: u32,
    pub spec_json: String,
}

impl Template {
    /// Guard: reject discard or edit on a Prebuilt template.
    pub fn guard_not_prebuilt(&self) -> Result<()> {
        if self.origin == TemplateOrigin::Prebuilt {
            Err(Error::PrebuiltImmutable { kind: "template" })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom_template() -> Template {
        Template {
            id: TemplateId::mint(),
            name: "my-template".to_owned(),
            origin: TemplateOrigin::Custom,
            pinned: false,
            spec_version: 1,
            spec_json: r#"{"items":[]}"#.to_owned(),
        }
    }

    fn prebuilt_template() -> Template {
        Template {
            origin: TemplateOrigin::Prebuilt,
            ..custom_template()
        }
    }

    #[test]
    fn guard_not_prebuilt_allows_custom_template() {
        let tmpl = custom_template();
        assert!(tmpl.guard_not_prebuilt().is_ok());
    }

    #[test]
    fn guard_not_prebuilt_rejects_prebuilt_template() {
        let tmpl = prebuilt_template();
        let err = tmpl.guard_not_prebuilt().unwrap_err();
        assert_eq!(err.code(), "prebuilt.immutable");
    }
}

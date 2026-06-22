//! Session aggregate: a product container of surfaces under a project.

use serde::{Deserialize, Serialize};

use super::project::ProjectId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct SessionId(String);

impl SessionId {
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

/// How a session's display title is derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, sqlx::Type)]
#[sqlx(rename_all = "kebab-case")]
pub enum TitleSource {
    /// Populated when the agent reports a title on completion.
    #[default]
    AgentTitle,
    /// Set to the git branch of the session root at creation time.
    Branch,
    /// Concatenation of branch (at creation) and agent title (when available).
    Both,
    /// Caller-supplied verbatim title.
    Custom,
}

impl TitleSource {
    pub fn as_str(self) -> &'static str {
        match self {
            TitleSource::AgentTitle => "agent-title",
            TitleSource::Branch => "branch",
            TitleSource::Both => "both",
            TitleSource::Custom => "custom",
        }
    }
}

/// Whether the session is active or archived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum SessionStatus {
    #[default]
    Active,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Session {
    pub id: SessionId,
    pub project_id: ProjectId,
    pub title: String,
    pub title_source: TitleSource,
    pub created_at: String,
    pub spec_version: Option<u32>,
    pub spec_json: Option<String>,
    pub sort_order: u32,
    pub pinned: bool,
    pub status: SessionStatus,
}

impl Session {
    /// Rename the session. Sets `title_source` to `Custom` so automatic titling
    /// does not override the user's choice.
    pub fn rename(&mut self, title: &str) {
        self.title = title.trim().to_owned();
        self.title_source = TitleSource::Custom;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active_session(id: &str) -> Session {
        Session {
            id: SessionId::from_string(id),
            project_id: ProjectId::new("p-1"),
            title: "My session".to_owned(),
            title_source: TitleSource::Branch,
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            spec_version: None,
            spec_json: None,
            sort_order: 0,
            pinned: false,
            status: SessionStatus::Active,
        }
    }

    #[test]
    fn rename_updates_title_and_sets_title_source_to_custom() {
        let mut s = active_session("s-1");
        s.rename("My custom title");
        assert_eq!(s.title, "My custom title");
        assert_eq!(s.title_source, TitleSource::Custom);
    }

    #[test]
    fn rename_trims_whitespace() {
        let mut s = active_session("s-1");
        s.rename("  spaced  ");
        assert_eq!(s.title, "spaced");
    }

    #[test]
    fn rename_sets_custom_even_when_previously_branch_title() {
        let mut s = active_session("s-1");
        assert_eq!(s.title_source, TitleSource::Branch);
        s.rename("override");
        assert_eq!(s.title_source, TitleSource::Custom);
    }
}

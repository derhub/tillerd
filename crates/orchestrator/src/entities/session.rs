//! Session aggregate: a product container of surfaces under a project.

use serde::{Deserialize, Serialize};

use super::project::ProjectId;
use crate::shared::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionStatus {
    #[default]
    Active,
    Archived,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SessionStatus::Active => "active",
            SessionStatus::Archived => "archived",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Guard: session must be Archived before hard-delete.
    pub fn guard_archived(&self) -> Result<()> {
        if self.status != SessionStatus::Archived {
            Err(Error::SessionNotArchived)
        } else {
            Ok(())
        }
    }

    /// Guard: session must be Active to archive.
    pub fn guard_active(&self) -> Result<()> {
        if self.status == SessionStatus::Archived {
            Err(Error::SessionAlreadyArchived)
        } else {
            Ok(())
        }
    }

    /// Archive-requires-idle guard. `live_surface_count` is the number of surfaces
    /// in `SurfaceStatus::Live` for this session, supplied by the caller (no I/O here).
    pub fn guard_idle(&self, live_surface_count: usize) -> Result<()> {
        if live_surface_count > 0 {
            Err(Error::SessionNotIdle(format!(
                "session {} has {live_surface_count} live surface(s)",
                self.id.as_str()
            )))
        } else {
            Ok(())
        }
    }
}

/// Parameters for creating a new session.
#[derive(Debug, Clone, Default)]
pub struct NewSession {
    pub project_id: Option<ProjectId>,
    pub title_source: TitleSource,
    /// Required when `title_source == Custom`; used as branch/agent-title for other strategies.
    pub title: Option<String>,
    /// When supplied, the session's spec blob and version are copied atomically from this template.
    pub template_id: Option<super::launch_template::LaunchTemplateId>,
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

    #[test]
    fn guard_idle_allows_session_with_no_live_surfaces() {
        let s = active_session("s-1");
        assert!(s.guard_idle(0).is_ok());
    }

    #[test]
    fn guard_idle_rejects_session_with_live_surfaces() {
        let s = active_session("s-1");
        assert!(s.guard_idle(2).is_err());
    }

    #[test]
    fn guard_archived_allows_archived_session() {
        let s = Session {
            status: SessionStatus::Archived,
            ..active_session("s-1")
        };
        assert!(s.guard_archived().is_ok());
    }

    #[test]
    fn guard_archived_rejects_active_session() {
        let s = active_session("s-1");
        assert!(s.guard_archived().is_err());
    }

    #[test]
    fn guard_active_allows_active_session() {
        let s = active_session("s-1");
        assert!(s.guard_active().is_ok());
    }

    #[test]
    fn guard_active_rejects_already_archived_session() {
        let s = Session {
            status: SessionStatus::Archived,
            ..active_session("s-1")
        };
        assert!(s.guard_active().is_err());
    }
}

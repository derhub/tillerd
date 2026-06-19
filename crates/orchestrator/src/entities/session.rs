//! Session aggregate: a product container of surfaces under a project.

use super::launch_template::LaunchTemplateId;
use super::project::ProjectId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

/// Parameters for creating a new session.
#[derive(Debug, Clone, Default)]
pub struct NewSession {
    pub project_id: Option<ProjectId>,
    pub title_source: TitleSource,
    /// Required when `title_source == Custom`; used as branch/agent-title for other strategies.
    pub title: Option<String>,
    /// When supplied, the session's spec blob and version are copied atomically from this template.
    pub template_id: Option<LaunchTemplateId>,
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
}

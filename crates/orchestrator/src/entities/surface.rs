//! Surface aggregate: a live pane (terminal/diff) under a session. Its id is the
//! correlation id shared with the daemon PTY and gate.

use serde::{Deserialize, Serialize};

use super::session::SessionId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct SurfaceId(String);

impl SurfaceId {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum SurfaceKind {
    Terminal,
    Diff,
}

impl SurfaceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SurfaceKind::Terminal => "terminal",
            SurfaceKind::Diff => "diff",
        }
    }
}

/// Lifecycle status of a surface (D9: persist intent -> effect -> record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum SurfaceStatus {
    /// Intent persisted; PTY spawn has not completed.
    #[default]
    Pending,
    /// PTY is running in the daemon.
    Live,
    /// Spawn failed or the PTY exited abnormally.
    Failed,
    /// PTY was stopped (SIGKILL via daemon); record kept for resume.
    Idle,
}

impl SurfaceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SurfaceStatus::Pending => "pending",
            SurfaceStatus::Live => "live",
            SurfaceStatus::Failed => "failed",
            SurfaceStatus::Idle => "idle",
        }
    }

    pub fn is_live(self) -> bool {
        self == SurfaceStatus::Live
    }
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Surface {
    pub id: SurfaceId,
    pub session_id: SessionId,
    pub kind: SurfaceKind,
    pub cwd: Option<String>,
    pub status: SurfaceStatus,
    pub placement: Option<String>,
}

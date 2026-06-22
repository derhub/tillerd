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

impl Surface {
    pub fn correlation_id(&self) -> &SurfaceId {
        &self.id
    }

    pub fn is_live(&self) -> bool {
        self.status.is_live()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending_surface() -> Surface {
        Surface {
            id: SurfaceId::from_string("surf-1"),
            session_id: SessionId::from_string("sess-1"),
            kind: SurfaceKind::Terminal,
            cwd: None,
            status: SurfaceStatus::Pending,
            placement: None,
        }
    }

    #[test]
    fn surface_status_pending_is_not_live() {
        let s = pending_surface();
        assert!(!s.is_live());
    }

    #[test]
    fn surface_status_live_is_live() {
        let s = Surface {
            status: SurfaceStatus::Live,
            ..pending_surface()
        };
        assert!(s.is_live());
    }

    #[test]
    fn surface_status_idle_is_not_live() {
        let s = Surface {
            status: SurfaceStatus::Idle,
            ..pending_surface()
        };
        assert!(!s.is_live());
    }

    #[test]
    fn surface_status_failed_is_not_live() {
        let s = Surface {
            status: SurfaceStatus::Failed,
            ..pending_surface()
        };
        assert!(!s.is_live());
    }

    #[test]
    fn correlation_id_matches_surface_id() {
        let s = pending_surface();
        assert_eq!(s.correlation_id(), &s.id);
    }
}

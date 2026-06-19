//! Surface aggregate: a live pane (terminal/diff) under a session. Its id is the
//! correlation id shared with the daemon PTY and gate.

use super::session::SessionId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
pub struct NewSurface {
    pub id: Option<SurfaceId>,
    pub session_id: SessionId,
    pub kind: SurfaceKind,
    pub cwd: Option<String>,
    pub placement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surface {
    pub id: SurfaceId,
    pub session_id: SessionId,
    pub kind: SurfaceKind,
    pub cwd: Option<String>,
    pub last_status: Option<String>,
    pub placement: Option<String>,
}

impl Surface {
    pub fn correlation_id(&self) -> &SurfaceId {
        &self.id
    }
}

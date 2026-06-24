//! Mode resolution: subcommand picks face, environment picks event source.

use std::path::{Path, PathBuf};

/// Canonical environment variable carrying the session id to subscribe to.
pub const SESSION_ID_ENV: &str = "TILLERD_SESSION_ID";

/// The face a subcommand serves.
#[derive(Debug, PartialEq, Eq)]
pub enum Face {
    /// The MCP stdio server plus the loopback viewer.
    McpWithViewer,
    ViewerOnly,
}

/// The face a subcommand serves, or `None` when the subcommand serves no face.
pub fn face_for_subcommand(cmd: &str) -> Option<Face> {
    match cmd {
        "mcp" => Some(Face::McpWithViewer),
        "serve" => Some(Face::ViewerOnly),
        _ => None,
    }
}

/// Where captured hook events come from.
#[derive(Debug, PartialEq, Eq)]
pub enum CaptureMode {
    /// Memory-only: no gate. Events come from an in-process stub (empty in
    /// production, fixed lists in tests).
    Standalone,
    /// Composed: subscribe to a gate's hook-event stream for one session.
    Composed {
        /// The gate's single socket, derived from the runtime directory; the
        /// subscriber opens it on the `Subscribe` route.
        subscribe_sock: PathBuf,
        /// The session to subscribe to.
        session_id: String,
    },
}

/// Resolve the capture mode from the environment.
pub fn capture_mode_from_env() -> CaptureMode {
    let base = tillerd_paths::runtime_dir();
    resolve_capture_mode(&base, std::env::var(SESSION_ID_ENV).ok().as_deref())
}

/// Resolve the capture mode from explicit values. A session id selects composed
/// capture, subscribing to the gate's subscribe socket under `base`; its absence
/// is standalone (the stub source).
pub fn resolve_capture_mode(base: &Path, session_id: Option<&str>) -> CaptureMode {
    match session_id {
        Some(id) if !id.is_empty() => CaptureMode::Composed {
            subscribe_sock: tillerd_paths::gate_socket_in(base),
            session_id: id.to_string(),
        },
        _ => CaptureMode::Standalone,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_subcommand_selects_standalone_mcp_plus_viewer() {
        assert_eq!(face_for_subcommand("mcp"), Some(Face::McpWithViewer));
    }

    #[test]
    fn serve_subcommand_selects_viewer_only() {
        assert_eq!(face_for_subcommand("serve"), Some(Face::ViewerOnly));
    }

    #[test]
    fn a_session_id_selects_gate_subscription_source() {
        assert_eq!(
            resolve_capture_mode(Path::new("/run/tillerd"), Some("s1")),
            CaptureMode::Composed {
                subscribe_sock: PathBuf::from("/run/tillerd/gate.sock"),
                session_id: "s1".to_string(),
            }
        );
    }

    #[test]
    fn no_session_id_selects_stub_source() {
        assert_eq!(
            resolve_capture_mode(Path::new("/run/tillerd"), None),
            CaptureMode::Standalone
        );
    }
}

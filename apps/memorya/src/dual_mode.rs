//! Startup mode resolution. The subcommand picks the face to serve; the
//! environment picks where captured events come from.

/// Canonical environment variable carrying the gate's subscribe socket path.
pub const GATE_URL_ENV: &str = "ATHING_GATE_URL";

/// Canonical environment variable carrying the session id to subscribe to.
pub const SESSION_ID_ENV: &str = "ATHING_SESSION_ID";

/// The face a subcommand serves.
#[derive(Debug, PartialEq, Eq)]
pub enum Face {
    /// The MCP stdio server plus the loopback viewer.
    McpWithViewer,
    /// The loopback viewer only.
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
        /// The gate's subscribe socket path.
        gate_url: String,
        /// The session to subscribe to.
        session_id: String,
    },
}

/// Resolve the capture mode from the environment.
pub fn capture_mode_from_env() -> CaptureMode {
    resolve_capture_mode(
        std::env::var(GATE_URL_ENV).ok().as_deref(),
        std::env::var(SESSION_ID_ENV).ok().as_deref(),
    )
}

/// Resolve the capture mode from explicit values. A non-empty gate url selects
/// composed capture (the gate-subscription source); its absence is standalone
/// (the stub source).
pub fn resolve_capture_mode(gate_url: Option<&str>, session_id: Option<&str>) -> CaptureMode {
    match gate_url {
        Some(url) if !url.is_empty() => CaptureMode::Composed {
            gate_url: url.to_string(),
            session_id: session_id.unwrap_or_default().to_string(),
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
    fn gate_url_present_selects_gate_subscription_source() {
        assert_eq!(
            resolve_capture_mode(Some("/run/gate.sock"), Some("s1")),
            CaptureMode::Composed {
                gate_url: "/run/gate.sock".to_string(),
                session_id: "s1".to_string(),
            }
        );
    }

    #[test]
    fn gate_url_absent_selects_stub_source() {
        assert_eq!(resolve_capture_mode(None, None), CaptureMode::Standalone);
    }
}

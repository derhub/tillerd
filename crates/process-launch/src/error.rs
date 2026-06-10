//! Launcher errors: typed by failure mode.

use thiserror::Error;

/// Failure modes of adopt-or-spawn and the restart loop.
#[derive(Debug, Error)]
pub enum LaunchError {
    /// The configured backend binary could not be resolved on disk.
    #[error("binary not found: {0}")]
    BinaryNotFound(String),

    /// The OS refused to spawn the backend process.
    #[error("spawn failed: {0}")]
    SpawnFailed(String),

    /// The backend did not become reachable within the startup timeout.
    #[error("startup timed out after {0} ms")]
    Timeout(u64),

    /// A live instance is running but its manifest version does not match the
    /// requested version (R3: adoption requires an exact version string match).
    #[error("version mismatch: running {running}, wanted {wanted}")]
    VersionMismatch {
        /// Version reported by the manifest of the live instance.
        running: String,
        /// Version the caller requires for adoption.
        wanted: String,
    },

    /// A control socket exists but did not accept a connection.
    #[error("socket unresponsive at {0}")]
    SocketUnresponsive(String),
}

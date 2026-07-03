//! Probes: injected I/O. Launcher logic is pure, side effects isolated.

use std::path::Path;
use std::time::Duration;

use crate::error::LaunchError;

/// Whether the process with `pid` is currently alive, via a signal-0 existence
/// check. Read-only: it delivers no signal. Shared by [`OsProbes`] and read-only
/// callers (e.g. health reads) that need liveness without the full probe set.
pub fn pid_is_alive(pid: u32) -> bool {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    // Signal 0 performs error checking without delivering a signal: Ok means
    // the process exists and can be signaled.
    kill(Pid::from_raw(pid as i32), None).is_ok()
}

/// The boundary operations adopt-or-spawn performs against the OS.
pub trait Probes {
    /// Whether the process with `pid` is currently alive.
    fn is_alive(&self, pid: u32) -> bool;

    /// Whether the control socket at `path` accepts a connection right now.
    fn is_reachable(&self, path: &Path) -> bool;

    /// Signal the process with `pid` to drain (SIGUSR2): refuse new work, finish active work, then
    /// exit. Retires a version-mismatched instance before starting the expected one.
    /// A no-op if the process is already gone.
    fn drain(&self, pid: u32);

    /// Remove the socket file at `path`, ignoring a missing file.
    fn remove_socket(&self, path: &Path);

    /// Spawn the backend; returns its pid. The child must write its own manifest
    /// once it is serving (the launcher only waits for socket reachability).
    fn spawn(&self) -> Result<u32, LaunchError>;

    /// Sleep for `dur` while polling for reachability.
    fn sleep(&self, dur: Duration);
}

/// Default probes backed by the live OS (real signals, sockets, clock).
pub struct OsProbes<F>
where
    F: Fn() -> Result<u32, LaunchError>,
{
    spawn_fn: F,
}

impl<F> OsProbes<F>
where
    F: Fn() -> Result<u32, LaunchError>,
{
    /// Build OS probes whose `spawn` delegates to `spawn_fn`.
    pub fn new(spawn_fn: F) -> Self {
        Self { spawn_fn }
    }
}

impl<F> Probes for OsProbes<F>
where
    F: Fn() -> Result<u32, LaunchError>,
{
    fn is_alive(&self, pid: u32) -> bool {
        pid_is_alive(pid)
    }

    fn is_reachable(&self, path: &Path) -> bool {
        std::os::unix::net::UnixStream::connect(path).is_ok()
    }

    fn drain(&self, pid: u32) {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGUSR2);
    }

    fn remove_socket(&self, path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    fn spawn(&self) -> Result<u32, LaunchError> {
        (self.spawn_fn)()
    }

    fn sleep(&self, dur: Duration) {
        std::thread::sleep(dur);
    }
}

//! Escalating graceful-then-forced shutdown of a tool's child processes.
//!
//! The host tracks every child it spawns. On a stop signal it asks each child to
//! terminate gracefully (`SIGTERM`), waits a bounded grace period, then forces
//! any survivor (`SIGKILL`) and reaps it. The result is that the host exits with
//! no orphaned children, honoring the reliability contract.

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::process::Child;

/// The grace period a child gets to exit after `SIGTERM` before it is forced.
pub const DEFAULT_GRACE_PERIOD: Duration = Duration::from_secs(5);

/// Tracks the children a host spawned so shutdown can terminate all of them.
#[derive(Clone, Default)]
pub struct ChildRegistry {
    children: Arc<Mutex<Vec<Child>>>,
}

impl ChildRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Track a spawned child so it is included in the shutdown sweep.
    pub fn track(&self, child: Child) {
        if let Ok(mut guard) = self.children.lock() {
            guard.push(child);
        }
    }

    /// The number of currently tracked children.
    pub fn len(&self) -> usize {
        self.children.lock().map(|g| g.len()).unwrap_or(0)
    }

    /// Whether no children are tracked.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Run the escalating shutdown: `SIGTERM`, grace, then `SIGKILL` survivors.
    ///
    /// Drains the registry; on return no tracked child remains running, so the
    /// host leaves no orphans.
    pub async fn shutdown_all(&self, grace: Duration) {
        let mut children: Vec<Child> = match self.children.lock() {
            Ok(mut guard) => std::mem::take(&mut *guard),
            Err(_) => return,
        };

        for child in children.iter_mut() {
            request_graceful(child);
        }

        let forced = tokio::time::timeout(grace, async {
            for child in children.iter_mut() {
                let _ = child.wait().await;
            }
        })
        .await;

        if forced.is_err() {
            for child in children.iter_mut() {
                // Survivors past the grace period are forced and reaped so the
                // host never exits with an orphan.
                let _ = child.start_kill();
                let _ = child.wait().await;
            }
        }
    }
}

/// Ask a child to terminate gracefully via `SIGTERM`.
fn request_graceful(child: &mut Child) {
    if let Some(pid) = child.id() {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_sleeper(seconds: u32) -> Child {
        tokio::process::Command::new("sleep")
            .arg(seconds.to_string())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn sleep")
    }

    fn is_alive(pid: u32) -> bool {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[tokio::test]
    async fn no_orphaned_children_after_shutdown() {
        let registry = ChildRegistry::new();
        let child = spawn_sleeper(60);
        let pid = child.id().expect("child pid");
        registry.track(child);
        assert!(is_alive(pid));

        registry.shutdown_all(DEFAULT_GRACE_PERIOD).await;

        // Give the OS a moment to reap, then confirm the child is gone.
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!is_alive(pid), "no child may outlive shutdown");
        assert!(registry.is_empty(), "registry drained after shutdown");
    }

    #[tokio::test]
    async fn escalation_to_sigkill_after_grace_period() {
        // A child that ignores SIGTERM must still be gone after escalation.
        let registry = ChildRegistry::new();
        let child = tokio::process::Command::new("/bin/sh")
            .args(["-c", "trap '' TERM; sleep 60"])
            .kill_on_drop(true)
            .spawn()
            .expect("spawn SIGTERM-ignoring child");
        let pid = child.id().expect("child pid");
        registry.track(child);

        // Wait for the trap to install.
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(is_alive(pid));

        // Short grace so the test escalates quickly.
        registry.shutdown_all(Duration::from_millis(300)).await;

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            !is_alive(pid),
            "a SIGTERM-ignoring child must be SIGKILLed after the grace period"
        );
    }
}

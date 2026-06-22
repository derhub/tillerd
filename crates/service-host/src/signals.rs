//! Lifecycle signals. SIGTERM / SIGINT -> graceful-then-forced teardown now (stop). SIGUSR2 -> drain
//! (refuse new work, finish active work, exit when idle -- design D1).

use tokio::signal::unix::{signal, SignalKind};

/// The graceful-termination signals the host listens for.
pub const GRACEFUL_TERMINATION_SIGNALS: [&str; 2] = ["SIGTERM", "SIGINT"];

/// The drain signal: refuse new work, let active work finish, exit when idle.
pub const DRAIN_SIGNAL: &str = "SIGUSR2";

/// Resolve on the first graceful-termination signal (`SIGTERM` or `SIGINT`).
///
/// Returns the name of the signal that fired so the caller can record it.
pub async fn wait_for_stop_signal() -> std::io::Result<&'static str> {
    let mut term = signal(SignalKind::terminate())?;
    let mut int = signal(SignalKind::interrupt())?;
    tokio::select! {
        _ = term.recv() => Ok("SIGTERM"),
        _ = int.recv() => Ok("SIGINT"),
    }
}

/// Resolve on the drain signal (`SIGUSR2`). Distinct from stop: the host flips the service into its
/// drain phase rather than tearing it down immediately.
pub async fn wait_for_drain_signal() -> std::io::Result<&'static str> {
    let mut usr2 = signal(SignalKind::user_defined2())?;
    usr2.recv().await;
    Ok(DRAIN_SIGNAL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graceful_termination_signals_listed() {
        assert!(GRACEFUL_TERMINATION_SIGNALS.contains(&"SIGTERM"));
        assert!(GRACEFUL_TERMINATION_SIGNALS.contains(&"SIGINT"));
    }

    #[tokio::test]
    async fn host_installs_signal_handlers_for_graceful_termination() {
        // Installing the handlers must succeed on the host platform; this is the
        // precondition for the host to react to a stop signal rather than die.
        assert!(signal(SignalKind::terminate()).is_ok());
        assert!(signal(SignalKind::interrupt()).is_ok());
    }

    #[tokio::test]
    async fn sigterm_triggers_graceful_shutdown_sequence() {
        // Send SIGTERM to ourselves and assert the wait future observes it and
        // names it, which is what kicks off the host's graceful sequence.
        let waiter = tokio::spawn(wait_for_stop_signal());
        // Give the handler a tick to install before raising the signal.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let pid = std::process::id().to_string();
        let status = std::process::Command::new("kill")
            .args(["-TERM", &pid])
            .status()
            .expect("kill command runs");
        assert!(status.success(), "kill -TERM self succeeded");

        let observed = tokio::time::timeout(std::time::Duration::from_secs(2), waiter)
            .await
            .expect("signal observed within timeout")
            .expect("task joined")
            .expect("signal future ok");
        assert_eq!(observed, "SIGTERM");
    }
}

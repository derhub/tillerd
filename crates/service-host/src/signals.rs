//! Graceful termination signals: SIGTERM / SIGINT → graceful-then-forced teardown.

use tokio::signal::unix::{signal, SignalKind};

/// The graceful-termination signals the host listens for.
pub const GRACEFUL_TERMINATION_SIGNALS: [&str; 2] = ["SIGTERM", "SIGINT"];

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

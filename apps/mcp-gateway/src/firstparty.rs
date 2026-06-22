//! First-party tools: orchestrator-supervised, never spawned by gateway.
//! instance -- is configured as an ordinary URL backend, which the gateway connects
//! to rather than spawns, so it is never rejected here.

use crate::config::BackendKind;

/// Tool names owned by the composition; the gateway refuses to spawn any of them.
pub const FIRST_PARTY: [&str; 5] = ["daemon", "gate", "gateway", "memorya", "memory"];

/// Whether `name` is a first-party (composition-owned) tool.
pub fn is_first_party(name: &str) -> bool {
    FIRST_PARTY.contains(&name)
}

/// A backend the gateway declined to spawn because it names a first-party tool.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("refusing to spawn first-party tool '{0}' as a gateway backend; it is launched by the orchestrator")]
pub struct FirstPartyRejected(pub String);

/// Reject spawning a process (stdio) backend that names a first-party tool. URL
/// backends are connected, not spawned, so they always pass.
pub fn reject_first_party_spawn(name: &str, kind: BackendKind) -> Result<(), FirstPartyRejected> {
    if matches!(kind, BackendKind::Stdio) && is_first_party(name) {
        return Err(FirstPartyRejected(name.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_five_first_party_names_are_recognized() {
        for name in ["daemon", "gate", "gateway", "memorya", "memory"] {
            assert!(is_first_party(name), "{name} should be first-party");
        }
    }

    #[test]
    fn external_backend_names_are_not_first_party() {
        assert!(!is_first_party("github"));
        assert!(!is_first_party("filesystem"));
    }

    #[test]
    fn spawning_a_first_party_process_backend_is_rejected() {
        assert!(reject_first_party_spawn("memory", BackendKind::Stdio).is_err());
        assert!(reject_first_party_spawn("daemon", BackendKind::Stdio).is_err());
        assert!(reject_first_party_spawn("gate", BackendKind::Stdio).is_err());
        assert!(reject_first_party_spawn("gateway", BackendKind::Stdio).is_err());
        assert!(reject_first_party_spawn("memorya", BackendKind::Stdio).is_err());
    }

    #[test]
    fn memory_as_an_ordinary_http_backend_is_not_rejected() {
        // Composed memory is reached as a URL backend: connected, never spawned.
        assert!(reject_first_party_spawn("memory", BackendKind::Http).is_ok());
    }

    #[test]
    fn external_process_backends_may_spawn() {
        assert!(reject_first_party_spawn("github", BackendKind::Stdio).is_ok());
    }
}

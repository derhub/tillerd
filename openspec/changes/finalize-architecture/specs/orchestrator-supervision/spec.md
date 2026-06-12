## ADDED Requirements

### Requirement: Version mismatch triggers drain-and-restart

The orchestrator SHALL drain a supervised service whose manifest version does not match
the expected version (drain signal, wait for the manifest to clear), swap in the
expected binary, and start it fresh. It SHALL NOT orchestrate any state handoff between
the old and new processes.

#### Scenario: Mismatched daemon is drained and replaced

- **WHEN** the orchestrator finds a running daemon whose manifest version mismatches
- **THEN** it signals drain, waits for the daemon's clean exit, and spawns the expected
  binary fresh

#### Scenario: Sessions resume after the restart

- **WHEN** the new daemon is ready after a drain-and-restart
- **THEN** previously persisted sessions are resumable through workspace persistence and
  the agent CLI's resume

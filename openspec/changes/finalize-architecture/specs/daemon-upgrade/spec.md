## ADDED Requirements

### Requirement: Drain-and-restart upgrade

On a planned upgrade the daemon SHALL drain: refuse new sessions, let active sessions
finish, then exit cleanly. The supervisor SHALL then start the new binary fresh. Session
continuity across the upgrade SHALL come from workspace persistence plus the agent CLI's
own resume, not from process handoff.

#### Scenario: Draining daemon refuses new sessions

- **WHEN** the daemon is draining and a client requests a new session
- **THEN** the request is refused with a typed error and active sessions are unaffected

#### Scenario: Daemon exits once active sessions finish

- **WHEN** the last active session on a draining daemon ends
- **THEN** the daemon exits cleanly and removes its manifest

#### Scenario: Explicit upgrade-now path

- **WHEN** the user chooses to upgrade immediately while sessions are active
- **THEN** active sessions are terminated deliberately and the daemon exits cleanly;
  no timer kills sessions automatically

## REMOVED Requirements

### Requirement: Zero-downtime daemon binary upgrade

**Reason**: fd-handoff is superseded by drain-and-restart; planned upgrades no longer
preserve live sessions (matches the ADR-0008 crash posture).
**Migration**: supervisor drains the old daemon, swaps the binary, starts fresh; resume
covers continuity.

### Requirement: Snapshot serialisation

**Reason**: existed solely to carry state across the fd-handoff; no successor process to
hand to.
**Migration**: workspace persistence is the only durable session state.

### Requirement: PTY fd inheritance via stdio

**Reason**: fd inheritance existed only for handoff; with it gone, no process needs to
inherit PTY masters.
**Migration**: none — the in-process-fd constraint is lifted, not replaced.

### Requirement: Upgrade-ack IPC handshake

**Reason**: predecessor/successor coordination disappears with the handoff itself.
**Migration**: the supervisor observes drain completion via the manifest lifecycle
status instead.

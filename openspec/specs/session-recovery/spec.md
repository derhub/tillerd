# session-recovery Specification

## Purpose
TBD - created by archiving change session-crash-recovery. Update Purpose after archive.
## Requirements
### Requirement: Stop operation

The engine SHALL expose a `stop()` operation distinct from `kill()`. `stop()` SHALL terminate the session and mark it as intentionally stopped, making it ineligible for client-initiated re-spawn. `kill()` SHALL terminate the session without preventing re-spawn.

#### Scenario: Stop prevents re-spawn

- **WHEN** `stop()` is invoked and the session exits
- **THEN** any subsequent `reconnect()` or re-spawn attempt for that session id SHALL be rejected with a typed `SessionStopped` error

#### Scenario: Kill does not prevent re-spawn

- **WHEN** `kill()` is invoked and the session exits
- **THEN** a caller MAY start a new session with `resume: sessionId` to continue

### Requirement: Client-initiated recovery via spawn

After a `crashed` status, a client SHALL be able to recover by starting a new session with `resume: sessionId`. Recovery SHALL launch a new agent process (the prior process is dead) with the agent's conversation-resume mechanism, reinstating the per-session lifecycle hook ingress for the new process. Recovery SHALL NOT attempt to reattach to the dead session. The engine SHALL NOT automatically re-spawn; recovery is always explicit.

#### Scenario: Recovery spawns a new process with conversation continuity

- **WHEN** `start({ resume: sessionId })` is called after a `crashed` status for that session id
- **THEN** the engine SHALL launch a new agent process that resumes the prior conversation, wire the per-session hook token for that process, and return a fresh session handle

#### Scenario: Recovery does not reattach to the dead session

- **WHEN** recovery is initiated for a crashed session
- **THEN** the engine SHALL NOT route through the live-session reattach path, because the prior process no longer exists

#### Scenario: Recovery wires hooks even when the adapter install is cached

- **WHEN** recovery spawns a new process for an adapter whose hook installation is already cached
- **THEN** the engine SHALL still wire the per-session hook token for the new process, so hook callbacks reach it

#### Scenario: No automatic re-spawn

- **WHEN** a session exits with a crash-class qualifier
- **THEN** the engine SHALL NOT automatically re-spawn the agent; it SHALL wait for an explicit client recovery call

### Requirement: Recovery does not restore pre-crash terminal state

Recovery SHALL restore conversation continuity through the agent's resume mechanism, not the rendered terminal screen. The new process begins with a blank terminal; pre-crash terminal scrollback SHALL NOT be replayed into the recovered session.

#### Scenario: Recovered terminal starts blank

- **WHEN** a crashed session is recovered via `start({ resume: sessionId })`
- **THEN** the new session's terminal SHALL begin empty and be populated only by output from the resumed agent, with no replay of pre-crash screen content

### Requirement: sessionId reuse on recovery

The recovered session SHALL reuse the same session id as the crashed session. Subscribers of the dead session SHALL NOT auto-reattach to the recovered session; a client SHALL drive a fresh subscribe.

#### Scenario: Stale subscribers do not auto-reattach

- **WHEN** a session is recovered under the same session id after a crash
- **THEN** subscribers of the dead session (already sent `crashed` and exit) SHALL NOT be auto-reattached; the client SHALL issue a fresh subscribe to receive the recovered session


## MODIFIED Requirements

### Requirement: Reconnect delivers replay buffer

The engine SHALL deliver a terminal state snapshot to the session handle immediately on reconnect, followed by the live data stream. The snapshot SHALL be emitted as a discrete frame on the data channel before any further data events, enabling the terminal renderer to restore the current screen without replaying raw byte history.

#### Scenario: Reconnect returns a working session handle

- **WHEN** `reconnect(sessionId, adapter, options)` is called for a session the daemon has live
- **THEN** the engine SHALL return an `AgentSession` that emits data, status, content, and error events identically to a session returned by `start`

#### Scenario: Reconnect delivers state snapshot

- **WHEN** `reconnect` is called for an existing session
- **THEN** the `AgentSession` SHALL emit a terminal state snapshot frame on the data channel before any new data events, so a terminal renderer can reproduce the current screen without prior history

#### Scenario: Reconnect to unknown session fails

- **WHEN** `reconnect` is called for a session id the daemon does not have
- **THEN** the engine SHALL reject with a typed error

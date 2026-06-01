## ADDED Requirements

### Requirement: Reconnect to existing session

The engine SHALL expose a `reconnect` operation that reattaches to a session already managed by the daemon, without spawning a new agent process, and returns an `AgentSession` handle with the same event model as a freshly started session.

#### Scenario: Reconnect returns a working session handle

- **WHEN** `reconnect(sessionId, adapter, options)` is called for a session the daemon has live
- **THEN** the engine SHALL return an `AgentSession` that emits data, status, content, and error events identically to a session returned by `start`

#### Scenario: Reconnect delivers replay buffer

- **WHEN** `reconnect` is called for an existing session
- **THEN** the `AgentSession` SHALL emit the replay buffer contents on the data channel before any new data events, so a terminal renderer can restore visual state

#### Scenario: Reconnect to unknown session fails

- **WHEN** `reconnect` is called for a session id the daemon does not have
- **THEN** the engine SHALL reject with a typed error

### Requirement: List daemon sessions

The engine SHALL expose a `listSessions` operation that returns the ids of all sessions currently alive in the daemon, so callers can determine which sessions are reconnectable.

#### Scenario: Returns live ids

- **WHEN** `listSessions()` is called
- **THEN** the engine SHALL return an array of session ids currently registered in the daemon

#### Scenario: Returns empty array when daemon has no sessions

- **WHEN** `listSessions()` is called and the daemon has no active sessions
- **THEN** the engine SHALL return an empty array

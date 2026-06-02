## MODIFIED Requirements

### Requirement: Per-session WebSocket connection

A WebSocket connection to `/ws/session` SHALL accept an optional session ID query parameter (`?id=<sessionId>`). When provided, the server SHALL attempt to reconnect to an existing session in the daemon. When absent, the server SHALL spawn a new session. Each connection is scoped to exactly one session for its lifetime.

- **WHEN** a WebSocket connection is opened with `?id=<sessionId>`
- **THEN** the server reconnects to the specified session and begins streaming its output to the client

#### Scenario: New session on bare connect

- **WHEN** a WebSocket connection is opened without an `id` parameter
- **THEN** the server spawns a new session, assigns it a session ID, and sends a `session_start` message

#### Scenario: Reconnect to existing session

- **WHEN** a WebSocket connection is opened with `?id=<sessionId>` for a session present in storage
- **THEN** the server reconnects to that session in the daemon and sends a `session_resume` message followed by buffered output

#### Scenario: Reconnect to unknown session

- **WHEN** a WebSocket connection is opened with `?id=<sessionId>` for a session not found in storage
- **THEN** the server sends an error message and closes the connection

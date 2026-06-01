## ADDED Requirements

### Requirement: Persistent session store

The server SHALL maintain a persistent store of active session metadata (id, working directory, creation timestamp) so that session ids survive server process restarts.

#### Scenario: Session recorded on start

- **WHEN** a new agent session is created
- **THEN** the server SHALL insert a record for it into the store before returning the session id to the client

#### Scenario: Session removed on exit

- **WHEN** a session's exit event fires
- **THEN** the server SHALL remove its record from the store

### Requirement: Startup reconciliation

On startup the server SHALL reconcile the persisted session records against the sessions the daemon reports as live, and SHALL remove records for sessions that the daemon no longer knows about.

#### Scenario: Stale record pruned

- **WHEN** the server starts and a persisted session id is absent from the daemon's live session list
- **THEN** the server SHALL delete that record from the store

#### Scenario: Live sessions retained

- **WHEN** the server starts and a persisted session id is present in the daemon's live session list
- **THEN** the server SHALL retain that record and make the session available for reconnect

### Requirement: Lazy reconnect via session id

The server SHALL reconnect to an existing daemon session only when a WebSocket client presents a known session id, not proactively on startup.

#### Scenario: Client reconnects with known id

- **WHEN** a WebSocket client connects and presents a session id that exists in both the store and the daemon
- **THEN** the server SHALL reattach to that daemon session, deliver the replay buffer to the client, and resume normal event forwarding

#### Scenario: Client presents unknown id

- **WHEN** a WebSocket client presents a session id not found in the store or not alive in the daemon
- **THEN** the server SHALL reject the reconnect with a typed error and close the connection

#### Scenario: Client connects without id

- **WHEN** a WebSocket client connects without a session id
- **THEN** the server SHALL start a new session as before

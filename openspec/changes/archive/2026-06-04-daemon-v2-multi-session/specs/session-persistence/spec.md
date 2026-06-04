## ADDED Requirements

### Requirement: Reconnect survives daemon upgrade

The server's reconnect contract SHALL remain valid across a daemon binary upgrade. After a successful upgrade the session ids recorded in the persistent store SHALL still be resolvable via the daemon's `list` response, and a WebSocket client presenting a known session id SHALL be able to reconnect and receive the replay buffer as normal.

#### Scenario: Session id stable across upgrade

- **WHEN** a daemon binary upgrade completes
- **THEN** a session id that was live before the upgrade SHALL appear in the daemon's `list` response after the upgrade

#### Scenario: Replay buffer available after upgrade

- **WHEN** a WebSocket client reconnects to a session that survived a daemon upgrade
- **THEN** the server SHALL deliver the replay buffer contents accumulated before and after the upgrade in order

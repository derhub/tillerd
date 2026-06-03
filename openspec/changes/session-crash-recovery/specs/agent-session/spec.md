## ADDED Requirements

### Requirement: Stop operation distinct from kill

The engine SHALL expose a `stop()` operation on the session contract. `stop()` SHALL terminate the session and mark it as intentionally stopped. A stopped session SHALL be ineligible for re-spawn via `resume`. `kill()` SHALL terminate the session without marking it as stopped.

#### Scenario: Stop marks session as stopped

- **WHEN** `stop()` is called on an active session
- **THEN** the engine SHALL terminate the session and record the stopped state against that session id

#### Scenario: Resume rejected for stopped session

- **WHEN** `start({ resume: sessionId })` is called for a session previously stopped via `stop()`
- **THEN** the engine SHALL reject with a typed `SessionStopped` error

#### Scenario: Kill allows resume

- **WHEN** `kill()` is called and the session exits
- **THEN** a caller MAY call `start({ resume: sessionId })` to recover

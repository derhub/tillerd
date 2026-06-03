## ADDED Requirements

### Requirement: Crashed session status

The engine SHALL emit a `crashed` status on the status channel when, and only when, a session exits with an exit qualifier that the shared qualifier-to-status mapping marks as a crash. The engine SHALL derive this solely from the platform-independent exit qualifier, never from raw platform exit codes or signals. `crashed` SHALL be a valid value in the `SessionStatus` contract alongside `IDLE`, `WORKING`, `WAITING_INPUT`, and `DONE`.

#### Scenario: Crash-class qualifier emits crashed status

- **WHEN** a session exits with a qualifier the mapping marks as a crash (for example `error` or `faulted`)
- **THEN** the engine SHALL emit status `crashed` before emitting the exit event

#### Scenario: Clean self-exit does not emit crashed

- **WHEN** a session exits with qualifier `ok`
- **THEN** the engine SHALL NOT emit status `crashed`

#### Scenario: Engine-initiated termination does not emit crashed

- **WHEN** a session exits with qualifier `stopped-by-request`
- **THEN** the engine SHALL NOT emit status `crashed`

## ADDED Requirements

### Requirement: Exit qualifier translation at the daemon boundary

The daemon SHALL translate every session exit into a single platform-independent exit qualifier and include it as the primary exit field in the exit event emitted over the IPC channel. The daemon SHALL be the only component that reads raw platform exit codes and signals for this purpose; it SHALL attach the raw code and signal only as optional diagnostic data. Translation precedence SHALL be: if a kill or stop command was received, `stopped-by-request`; else for a signal-free exit, `ok` for code zero and `error` otherwise; else the terminating signal's category SHALL select the matching qualifier; else `unknown`.

#### Scenario: Kill or stop command yields stopped-by-request

- **WHEN** a kill or stop command is received for a session and the session subsequently exits
- **THEN** the exit event emitted by the daemon SHALL carry qualifier `stopped-by-request` regardless of the underlying exit code or signal

#### Scenario: Zero-code self-exit yields ok

- **WHEN** a session process exits with code zero and no signal without a preceding kill or stop command
- **THEN** the exit event emitted by the daemon SHALL carry qualifier `ok`

#### Scenario: Non-zero self-exit yields error

- **WHEN** a session process exits with a non-zero code and no signal without a preceding kill or stop command
- **THEN** the exit event emitted by the daemon SHALL carry qualifier `error`

#### Scenario: Signal exit maps by category

- **WHEN** a session process is terminated by a signal without a preceding kill or stop command
- **THEN** the daemon SHALL map the signal's category to the matching qualifier (for example a fault-category signal to `faulted`) and SHALL preserve the raw signal as diagnostic data

#### Scenario: External forced kill is distinct from a requested stop

- **WHEN** a session is terminated by a forced-termination signal with no preceding kill or stop command
- **THEN** the exit event SHALL carry qualifier `killed`, distinct from `stopped-by-request`

#### Scenario: Raw values are diagnostic only

- **WHEN** the daemon emits any exit event
- **THEN** the platform exit code and signal SHALL appear only as optional diagnostic fields and the qualifier SHALL be the primary exit field

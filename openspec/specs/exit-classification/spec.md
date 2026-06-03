# exit-classification Specification

## Purpose
Defines the closed, platform-independent exit taxonomy. The daemon translates each raw process exit (code, signal) into a single `ExitQualifier` at its boundary; every layer above branches only on the qualifier, never on raw platform values. Includes the signal reference table with platform-stable name resolution and the qualifier-to-status mapping that determines the `crashed` status.

## Requirements
### Requirement: Platform-independent exit qualifier

Every session exit SHALL be translated, at the daemon boundary, into a single value from a closed, platform-independent exit qualifier set, and that qualifier SHALL be the exit field consumers branch on. Raw platform exit code and signal SHALL be carried only as optional diagnostic data, never as the basis for downstream control flow. The qualifier set SHALL include at least: `ok`, `error`, `stopped-by-request`, `killed`, `faulted`, `hangup`, `interrupted`, `resource-exceeded`, and `unknown`.

#### Scenario: Raw platform values do not cross the boundary as logic

- **WHEN** a session exits by any means
- **THEN** the exit event SHALL carry a platform-independent qualifier as its primary exit field, and any raw exit code or signal SHALL appear only as optional diagnostic data

#### Scenario: Engine-initiated termination qualifies as stopped-by-request

- **WHEN** a kill or stop command is issued and the session subsequently exits
- **THEN** the exit qualifier SHALL be `stopped-by-request` regardless of the underlying platform code or signal

#### Scenario: Zero-code self-exit qualifies as ok

- **WHEN** the session exits with code zero and no signal and no kill or stop command was issued
- **THEN** the exit qualifier SHALL be `ok`

#### Scenario: Non-zero self-exit qualifies as error

- **WHEN** the session exits with a non-zero code and no signal and no kill or stop command was issued
- **THEN** the exit qualifier SHALL be `error`

#### Scenario: Program fault qualifies as faulted

- **WHEN** the session is terminated by a fault-category signal with no preceding kill or stop command
- **THEN** the exit qualifier SHALL be `faulted`

#### Scenario: Unmapped exit qualifies as unknown

- **WHEN** the session exits in a way that maps to no defined qualifier
- **THEN** the exit qualifier SHALL be `unknown` and the raw values SHALL be preserved as diagnostic data

### Requirement: Qualifier-driven status mapping

The mapping from exit qualifier to session status SHALL be defined once and SHALL be the sole determinant of whether an exit produces a `crashed` status. Consumers SHALL NOT re-derive crash detection from raw platform values.

#### Scenario: Single source of truth for crashed

- **WHEN** any consumer needs to know whether an exit is a crash
- **THEN** it SHALL derive that solely from the exit qualifier via the shared mapping, not from the raw exit code or signal

#### Scenario: Coarse classification derives from qualifier

- **WHEN** a coarse `user` / `clean` / `unexpected` classification is needed
- **THEN** it SHALL be derived as a grouping of the qualifier, not computed independently

### Requirement: Signal reference table with platform-stable resolution

The SDK SHALL provide a single signal reference table mapping each standard signal name to a human-readable meaning and a category. When the pseudo-terminal binding reports a terminating signal as a platform-specific number, the number SHALL be resolved to a signal name via the platform's number-to-name map before meaning and category are attached, so the same signal yields the same name on every supported platform. A signal absent from the table SHALL be reported as unknown with its raw value preserved.

#### Scenario: Signal carries name, meaning, and category

- **WHEN** a session is terminated by a signal
- **THEN** the exit event SHALL include the signal's resolved name, its meaning, and its category drawn from the reference table

#### Scenario: Signal number resolved across platforms

- **WHEN** the binding reports a terminating signal as a platform-specific number
- **THEN** the number SHALL be resolved to a signal name via the platform's number-to-name map before lookup, so the same signal yields the same name on every supported platform

#### Scenario: Unknown signal preserved

- **WHEN** a session is terminated by a signal not present in the reference table
- **THEN** the exit event SHALL preserve the raw signal value and report it as unknown

### Requirement: Crashed status from crash-class qualifiers only

When a session exits with a qualifier the shared mapping marks as a crash, the engine SHALL emit a `crashed` status event. For qualifiers `ok` and `stopped-by-request`, the engine SHALL NOT emit `crashed`. `crashed` SHALL be a first-class value in the session status contract.

#### Scenario: Crash-class qualifier emits crashed

- **WHEN** a session exits with a qualifier the mapping marks as a crash (for example `error`, `faulted`, `killed`, `hangup`, `interrupted`, `resource-exceeded`, or `unknown`)
- **THEN** the engine SHALL emit status `crashed` on the status channel before emitting the exit event

#### Scenario: Clean self-exit does not emit crashed

- **WHEN** a session exits with qualifier `ok`
- **THEN** the engine SHALL NOT emit status `crashed`

#### Scenario: Engine-initiated termination does not emit crashed

- **WHEN** a session exits with qualifier `stopped-by-request`
- **THEN** the engine SHALL NOT emit status `crashed`


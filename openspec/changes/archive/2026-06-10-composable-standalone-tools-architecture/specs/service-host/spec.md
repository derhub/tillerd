## ADDED Requirements

### Requirement: Uniform lifecycle entry

Every long-lived tool SHALL start through a single host entry point that performs resource-path
resolution, manifest lifecycle, signal handling, and shutdown on the tool's behalf. The tool
SHALL supply only its identity and its serve behavior; it SHALL NOT reimplement the plumbing.

#### Scenario: Tool starts via the host entry point

- **WHEN** a tool is started
- **THEN** the host SHALL resolve its resource paths, write its manifest, install signal handlers, and invoke the tool's serve behavior

### Requirement: Deterministic resource paths

The host SHALL resolve a tool's base directory and derive its manifest and socket paths
deterministically, honoring a base-directory override so multiple isolated instances can coexist.

#### Scenario: Paths derived from the base directory

- **WHEN** the host resolves paths for a tool
- **THEN** the manifest and socket paths SHALL be derived deterministically from the resolved base directory

#### Scenario: Base-directory override respected

- **WHEN** a base-directory override is supplied
- **THEN** all derived paths SHALL be rooted at the overridden directory

### Requirement: Atomic manifest lifecycle

The host SHALL write a tool's manifest carrying at least its process identity and version, using a
write-then-rename so readers never observe a partial file, and SHALL remove the manifest on clean
stop.

#### Scenario: Manifest written atomically on start

- **WHEN** a tool starts
- **THEN** its manifest SHALL be written via a temporary file and atomic rename
- **AND** it SHALL carry the process identity and version

#### Scenario: Manifest removed on clean stop

- **WHEN** a tool stops cleanly
- **THEN** its manifest SHALL be removed

### Requirement: Signal-driven graceful shutdown

On a stop signal the host SHALL run an escalating graceful-then-forced shutdown, release the
tool's resources, and exit with no orphaned children, honoring the reliability contract.

#### Scenario: Graceful shutdown on signal

- **WHEN** a tool receives a stop signal
- **THEN** the host SHALL shut the tool down with escalation and exit with no orphaned children

### Requirement: Liveness probe

A tool started through the host SHALL expose a cheap, unauthenticated reachability check so a
launcher can detect a running instance and its version before holding any credential.

#### Scenario: Launcher probes before connecting

- **WHEN** a launcher probes a tool's liveness endpoint
- **THEN** the tool SHALL report reachability and version without requiring a credential

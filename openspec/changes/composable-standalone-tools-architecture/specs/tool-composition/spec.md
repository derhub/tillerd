## ADDED Requirements

### Requirement: Standalone operability

Each tool SHALL run as a standalone process with no required peer. When a peer is absent, the
tool SHALL degrade its function gracefully and SHALL NOT crash.

#### Scenario: Tool runs with no peers

- **WHEN** a tool is started alone
- **THEN** it SHALL operate within the limits of its own concern without requiring any other tool

#### Scenario: Absent peer degrades, not crashes

- **WHEN** a feature that needs an absent peer is exercised
- **THEN** the tool SHALL report the limitation and continue running

### Requirement: Launch authority

A first-party tool SHALL be launched only by an orchestrator. No first-party tool SHALL spawn
another first-party tool. The tool gateway MAY spawn only external (non-first-party) tool
backends.

#### Scenario: Orchestrator launches tools

- **WHEN** a composition is started
- **THEN** the orchestrator SHALL launch each selected first-party tool

#### Scenario: Tool does not spawn a peer tool

- **WHEN** a first-party tool needs another first-party tool
- **THEN** it SHALL connect to the already-running instance and SHALL NOT spawn it

### Requirement: Contract-only coupling

Tools SHALL communicate only through published contracts. A tool SHALL NOT depend on another
tool's internals, nor on an orchestrator.

#### Scenario: Coupling is through contracts

- **WHEN** one tool consumes another
- **THEN** it SHALL do so only through a published contract surface

#### Scenario: No dependency on orchestrators

- **WHEN** a tool's dependencies are enumerated
- **THEN** they SHALL include no orchestrator

### Requirement: Dual-mode ports

A tool able to run standalone or composed SHALL select its input source and its exposure through a
port wired externally, not through internal hard-coding of a single mode.

#### Scenario: Source selected by wiring

- **WHEN** a dual-mode tool is composed
- **THEN** its input source SHALL be the one wired for the composition, not a hard-coded default

### Requirement: Composition-aware wiring

The orchestrator SHALL wire each composition's ingress source, backend registration, and
hook-install target according to the selected tools.

#### Scenario: Hook-install target matches the wiring

- **WHEN** a composition is established
- **THEN** the installed agent hook SHALL target the ingress chosen for that composition

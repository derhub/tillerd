# terminal-status Specification

## Purpose

TBD - created by syncing change terminal-status. Update Purpose after archive.

## Requirements

### Requirement: Terminal status as a first-class session signal

The system SHALL publish, for each session, a terminal status derived from the operating system's view of its process, exposed independently of and alongside the agent's hook-derived lifecycle status.

#### Scenario: Status emitted on transition with terminal source

- **GIVEN** a running session
- **WHEN** the terminal status changes
- **THEN** the backend SHALL emit a status message carrying the session id, a status value of either `IDLE` or `WORKING`, and a `source` field identifying the value as terminal-derived
- **AND** the message SHALL be emitted only on transition, not on every sampling tick
- **AND** the terminal status SHALL be published in its own right — neither suppressed by, nor a substitute for, the agent's hook-derived status

### Requirement: Terminal status and agent status are distinct planes

The system SHALL treat terminal status and agent status as two independent answers to two different questions — process reality versus agent intent — without one masking the other.

#### Scenario: Both statuses available and source-tagged

- **GIVEN** a session that exposes both an agent (hook-derived) status and a terminal status
- **WHEN** a consumer observes the session
- **THEN** both statuses SHALL be available and each SHALL be tagged by its `source`
- **AND** neither status SHALL be defined as authoritative over the other; they describe different facts
- **AND** combining the two into a single displayed value, if a consumer chooses to, SHALL be a presentation concern outside this capability

### Requirement: WORKING vs IDLE from terminal facts

The system SHALL derive `WORKING`/`IDLE` from observable terminal facts so that terminal status reflects whether the session's process is actively doing work without inspecting screen content.

#### Scenario: A job other than the root holds the foreground group

- **GIVEN** a session whose foreground process group is held by a job other than the session root
- **WHEN** terminal status is computed
- **THEN** the terminal status SHALL be `WORKING`, since a job is actively running

#### Scenario: Root holds the foreground group and output is quiet

- **GIVEN** the session root holds the foreground process group
- **AND** no output has been produced for a configured quiescence threshold
- **WHEN** terminal status is computed
- **THEN** the terminal status SHALL be `IDLE`, denoting "awaiting the user / ready for the next prompt"

#### Scenario: Output is being produced within the quiescence threshold

- **GIVEN** output is being produced within the quiescence threshold
- **WHEN** terminal status is computed
- **THEN** the terminal status SHALL be `WORKING`
- **AND** terminal status SHALL draw only from the existing fixed status set and introduce no new status value

### Requirement: Terminal status does not assert WAITING_INPUT

The system SHALL limit terminal status to `IDLE` and `WORKING` so that no semantic input-waiting state is inferred from terminal facts that cannot reliably establish one.

#### Scenario: Terminal facts never yield WAITING_INPUT

- **GIVEN** terminal facts only — output activity and the foreground process group
- **WHEN** the backend computes terminal status
- **THEN** the emitted value SHALL be limited to `IDLE` or `WORKING`, and never `WAITING_INPUT`
- **AND** `WAITING_INPUT` SHALL remain a property of the agent's hook-derived status, since "the foreground process holds the terminal and is quiet" is indistinguishable, from terminal facts alone, between "ready for input" and "blocked on a specific answer"
- **AND** richer terminal-derived input-waiting detection (for example operating-system read-block introspection) SHALL be deferred to future work

### Requirement: Completion is observed through the existing exit signal

The system SHALL leave session completion to the existing process-exit signal so that there is a single, precisely classified source of completion rather than a duplicate.

#### Scenario: Completion reported on the exit signal, not as a status

- **GIVEN** a session whose root process exits
- **WHEN** the backend reaps the exit
- **THEN** completion SHALL be reported through the existing exit signal that already carries the exit facts and their classification (including abnormal-termination classifications)
- **AND** this terminal-status capability SHALL NOT emit a separate completion status
- **AND** terminal status sampling SHALL stop once the session has exited

### Requirement: Initial status and delivery on subscribe

The system SHALL establish an initial terminal status and deliver the current terminal status immediately to a client that subscribes mid-session.

#### Scenario: Initial status established on spawn

- **GIVEN** a newly spawned session
- **WHEN** the session begins
- **THEN** an initial terminal status SHALL be established and available

#### Scenario: Current status delivered on subscribe

- **GIVEN** a client subscribes to a session that is already running
- **WHEN** the subscription is established
- **THEN** the client SHALL receive the session's current terminal status without waiting for the next transition

### Requirement: Terminal status works for adopted sessions and unknown foreground groups

The system SHALL compute terminal status correctly for sessions adopted across a daemon handoff and when the foreground process group cannot be read, so that the signal is robust across the daemon lifecycle.

#### Scenario: Adopted session without a reaper

- **GIVEN** a session adopted from a predecessor daemon over an inherited pseudo-terminal, with no owning child reaper
- **WHEN** terminal status is computed
- **THEN** `WORKING` / `IDLE` SHALL still be derived from output activity and the foreground process group as available

#### Scenario: Foreground process group cannot be determined

- **GIVEN** the foreground process group of the pseudo-terminal cannot be determined
- **WHEN** terminal status is computed
- **THEN** the backend SHALL degrade to output-quiescence alone and SHALL NOT fail

### Requirement: Terminal status is coarse and does not corrupt existing consumers

The system SHALL route terminal status as its own signal so that existing logic driven by the agent status is unaffected.

#### Scenario: Existing agent-status consumers ignore terminal-source messages

- **GIVEN** existing logic that consumes the agent (hook-derived) status, such as idle-timeout and prompt-queue handling
- **WHEN** terminal status messages are emitted
- **THEN** that existing logic SHALL continue to consume only the agent status and SHALL NOT be driven by terminal-`source` messages

#### Scenario: Continuous redraw may remain WORKING

- **GIVEN** a session that continuously redraws output, such as an animated progress indicator
- **WHEN** terminal status is computed from output quiescence
- **THEN** the status MAY remain `WORKING` while the redraw continues, and this SHALL be accepted as a known limitation

#### Scenario: Long-lived full-screen program toggles on quiescence alone

- **GIVEN** a single long-lived full-screen program that always holds the foreground process group
- **WHEN** terminal status is computed
- **THEN** `WORKING` versus `IDLE` SHALL be decided by the output-quiescence threshold alone, since the foreground process group does not change

### Requirement: Terminal status frame is defined in the shared wire contract

The system SHALL define the terminal status frame once in the shared server-to-client contract so that any client parses it uniformly and any daemon implementation that emits it stays consistent.

#### Scenario: Frame added to the typed contract and its validation

- **GIVEN** the shared server-to-client frame contract
- **WHEN** the terminal status frame is added
- **THEN** it SHALL be included in the typed frame set and its validation so clients can parse it
- **AND** a daemon implementation that emits the frame SHALL produce field names and values matching the contract
- **AND** mirroring emission in additional daemon implementations SHALL be future work: where a runtime cannot read the foreground process group natively, the implementation MAY omit emission rather than diverge from the contract

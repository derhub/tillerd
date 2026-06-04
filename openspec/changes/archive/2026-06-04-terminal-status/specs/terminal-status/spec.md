## ADDED User Stories

### User Story: Terminal status as a first-class session signal

As a UI integrator, I want each session to publish a terminal status derived from the operating system's view of its process, so that I can observe process reality directly — independent of, and alongside, the agent's hook-derived lifecycle status.

#### Acceptance Criteria

- **Given** a running session
- **When** the terminal status changes
- **Then** the backend emits a status message carrying the session id, a status value of either `IDLE` or `WORKING`, and a `source` field identifying the value as terminal-derived
- **And** the message is emitted only on transition, not on every sampling tick
- **And** the terminal status is published in its own right — it is neither suppressed by, nor a substitute for, the agent's hook-derived status.

### User Story: Terminal status and agent status are distinct planes

As a consumer, I want terminal status and agent status treated as two independent answers to two different questions, so that I can use whichever fits — process reality versus agent intent — without one masking the other.

#### Acceptance Criteria

- **Given** a session that exposes both an agent (hook-derived) status and a terminal status
- **When** a consumer observes the session
- **Then** both statuses are available and each is tagged by its `source`
- **And** neither status is defined as authoritative over the other; they describe different facts
- **And** combining the two into a single displayed value, if a consumer chooses to, is a presentation concern outside this capability.

### User Story: WORKING vs IDLE from terminal facts

As a client, I want `WORKING`/`IDLE` derived from observable terminal facts, so that terminal status reflects whether the session's process is actively doing work without inspecting screen content.

#### Acceptance Criteria

- **Given** a session whose foreground process group is held by a job other than the session root
- **When** terminal status is computed
- **Then** the terminal status is `WORKING`, since a job is actively running
- **Given** the session root holds the foreground process group
- **And** no output has been produced for a configured quiescence threshold
- **Then** the terminal status is `IDLE`, denoting "awaiting the user / ready for the next prompt"
- **Given** output is being produced within the quiescence threshold
- **Then** the terminal status is `WORKING`
- **And** terminal status draws only from the existing fixed status set and introduces no new status value.

### User Story: Terminal status does not assert WAITING_INPUT

As an integrator, I want terminal status to limit itself to `IDLE` and `WORKING`, so that no semantic input-waiting state is inferred from terminal facts that cannot reliably establish one.

#### Acceptance Criteria

- **Given** terminal facts only — output activity and the foreground process group
- **When** the backend computes terminal status
- **Then** the emitted value is limited to `IDLE` or `WORKING`, and never `WAITING_INPUT`
- **And** `WAITING_INPUT` remains a property of the agent's hook-derived status, since "the foreground process holds the terminal and is quiet" is indistinguishable, from terminal facts alone, between "ready for input" and "blocked on a specific answer"
- **And** richer terminal-derived input-waiting detection (for example operating-system read-block introspection) is explicitly deferred to future work.

### User Story: Completion is observed through the existing exit signal

As a client, I want session completion to remain the responsibility of the existing process-exit signal, so that there is a single, precisely classified source of completion rather than a duplicate.

#### Acceptance Criteria

- **Given** a session whose root process exits
- **When** the backend reaps the exit
- **Then** completion is reported through the existing exit signal that already carries the exit facts and their classification (including abnormal-termination classifications)
- **And** this terminal-status capability does NOT emit a separate completion status
- **And** terminal status sampling stops once the session has exited.

### User Story: Initial status and delivery on subscribe

As a client that subscribes mid-session, I want the current terminal status delivered immediately, so that I am not blind until the next transition.

#### Acceptance Criteria

- **Given** a newly spawned session
- **When** the session begins
- **Then** an initial terminal status is established and available
- **Given** a client subscribes to a session that is already running
- **When** the subscription is established
- **Then** the client receives the session's current terminal status without waiting for the next transition.

### User Story: Terminal status works for adopted sessions and unknown foreground groups

As an operator, I want terminal status to behave correctly for sessions adopted across a daemon handoff and when the foreground process group cannot be read, so that the signal is robust across the daemon lifecycle.

#### Acceptance Criteria

- **Given** a session adopted from a predecessor daemon over an inherited pseudo-terminal, with no owning child reaper
- **When** terminal status is computed
- **Then** `WORKING` / `IDLE` are still derived from output activity and the foreground process group as available
- **Given** the foreground process group of the pseudo-terminal cannot be determined
- **When** terminal status is computed
- **Then** the backend degrades to output-quiescence alone and does not fail.

### User Story: Terminal status is coarse and does not corrupt existing consumers

As an integrator, I want terminal status routed as its own signal, so that existing logic driven by the agent status is unaffected.

#### Acceptance Criteria

- **Given** existing logic that consumes the agent (hook-derived) status, such as idle-timeout and prompt-queue handling
- **When** terminal status messages are emitted
- **Then** that existing logic continues to consume only the agent status and is not driven by terminal-`source` messages
- **Given** a session that continuously redraws output, such as an animated progress indicator
- **When** terminal status is computed from output quiescence
- **Then** the status may remain `WORKING` while the redraw continues, and this is accepted as a known limitation
- **Given** a single long-lived full-screen program that always holds the foreground process group
- **When** terminal status is computed
- **Then** `WORKING` versus `IDLE` is decided by the output-quiescence threshold alone, since the foreground process group does not change.

### User Story: Terminal status frame is defined in the shared wire contract

As an operator, I want the terminal status frame defined once in the shared server-to-client contract, so that any client parses it uniformly and any daemon implementation that emits it stays consistent.

#### Acceptance Criteria

- **Given** the shared server-to-client frame contract
- **When** the terminal status frame is added
- **Then** it is included in the typed frame set and its validation so clients can parse it
- **And** a daemon implementation that emits the frame produces field names and values matching the contract
- **And** mirroring emission in additional daemon implementations is future work: where a runtime cannot read the foreground process group natively, the implementation may omit emission rather than diverge from the contract.

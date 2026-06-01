# agent-session

## Purpose

Defines the engine's session model: factory isolation, the session lifecycle contract, event model, reliability guarantees, and observability. The engine holds no global state; all resources are instance-scoped.

## Requirements

### Requirement: Engine instances with no global state

The engine SHALL be created via a factory that returns an isolated instance owning its own session registry and resources. The engine SHALL NOT hold module-level mutable state, so multiple engine instances and many concurrent sessions can coexist in one host process.

#### Scenario: Independent instances

- **WHEN** two engine instances are created in the same process
- **THEN** each SHALL manage its own sessions and resources without interfering with the other

#### Scenario: Shutdown is instance-scoped

- **WHEN** one engine instance is shut down
- **THEN** only that instance's sessions and resources SHALL be released, leaving any other instance running

### Requirement: Session lifecycle

The engine SHALL expose a session contract with `start`, `send`, `input`, `interrupt`, `resize`, `kill`, and `resume`, uniform across transport modes.

#### Scenario: Start a session

- **WHEN** a caller invokes `start(agentDefinition, options)`
- **THEN** the engine SHALL launch the agent, return a session handle carrying the session id, and begin emitting events

#### Scenario: Terminate a session

- **WHEN** a caller invokes `kill()`
- **THEN** the engine SHALL terminate the agent and release that session's resources

### Requirement: Ready-gated prompt submission

A session SHALL be considered ready for prompts only after it reaches the IDLE status. `send` issued before ready, or while the session is WORKING, SHALL be queued until the next IDLE; the queue SHALL be bounded and overflow SHALL produce a typed error.

#### Scenario: First prompt waits for ready

- **WHEN** `send(text)` is called before the session has reached IDLE
- **THEN** the engine SHALL hold the prompt and deliver it once the session becomes ready

#### Scenario: Send while working is queued

- **WHEN** `send(text)` is called while the session is WORKING
- **THEN** the engine SHALL queue the prompt and deliver it at the next IDLE

#### Scenario: Queue overflow

- **WHEN** queued prompts exceed the bounded queue capacity
- **THEN** the engine SHALL reject the excess with a typed error rather than grow unbounded

### Requirement: Interrupt versus kill

`interrupt` SHALL cancel the current turn while keeping the session alive; `kill` SHALL terminate the session.

#### Scenario: Interrupt keeps the session

- **WHEN** `interrupt()` is called during a WORKING turn
- **THEN** the engine SHALL cancel the in-progress turn and the session SHALL remain usable for further prompts

### Requirement: Canonical event model

The engine SHALL emit a single normalized event model with three kinds — raw terminal data, status, and structured content — regardless of transport mode.

#### Scenario: Data events

- **WHEN** the agent produces terminal output
- **THEN** the engine SHALL emit it on the data channel as unmodified bytes

#### Scenario: Status events

- **WHEN** the agent's lifecycle state changes
- **THEN** the engine SHALL emit a normalized status on the status channel

#### Scenario: Content events

- **WHEN** structured content is derived for the session
- **THEN** the engine SHALL emit typed content events on the content channel

### Requirement: HookEvent is the lifecycle entry point

The engine SHALL consume agent lifecycle exclusively as a normalized `HookEvent` value (`{ sessionId, type, payload? }`). Status and content logic SHALL depend only on `HookEvent`, not on how the event was received.

#### Scenario: Dispatch drives status and content

- **WHEN** a `HookEvent` is dispatched to the engine
- **THEN** the engine SHALL route it by session id and update status and content from it, without any knowledge of the transport that produced it

### Requirement: Raw byte fidelity

The engine SHALL preserve agent terminal output as raw bytes end-to-end, with no ANSI stripping and no intermediate text re-decode.

#### Scenario: Bytes pass through unmodified

- **WHEN** the agent emits output containing ANSI escapes and multibyte UTF-8 characters
- **THEN** the bytes delivered on the data channel SHALL be byte-for-byte identical to what the agent wrote

### Requirement: Raw input and resize

The engine SHALL accept raw input bytes and terminal resize requests and forward them to the running agent.

#### Scenario: Forward raw keystrokes

- **WHEN** a caller invokes `input(bytes)`
- **THEN** the engine SHALL write those exact bytes to the agent's input channel without gating

#### Scenario: Propagate resize

- **WHEN** a caller invokes `resize(cols, rows)`
- **THEN** the engine SHALL propagate the new dimensions to the agent's terminal

### Requirement: Graceful shutdown and exit capture

On termination the engine SHALL escalate from a graceful stop signal to a forced kill after a grace period, capture the agent's exit code and signal, emit a terminal exit event, and clean up all session resources on both normal exit and crash.

#### Scenario: Escalating termination

- **WHEN** `kill()` is called and the agent does not exit within the grace period
- **THEN** the engine SHALL force-terminate it, emit an exit event with the code/signal, and leave no orphaned process or terminal

### Requirement: Bounded interactions via timeouts

Every external interaction SHALL be time-bounded — at minimum startup, shutdown grace, and idle — and a timeout SHALL produce a typed error and a defined transition rather than an indefinite hang.

#### Scenario: Startup timeout

- **WHEN** the agent does not become ready within the startup timeout
- **THEN** the engine SHALL emit a typed timeout error and terminate the session

### Requirement: Typed error taxonomy

Errors SHALL be a closed, typed set (including `BinaryNotFound`, `NotAuthenticated`, `SpawnFailed`, `HookInstallFailed`, `TranscriptUnavailable`, `TransportClosed`, `Timeout`, `VersionUnsupported`) surfaced on the event model so callers can branch on them.

#### Scenario: Errors are typed

- **WHEN** the agent binary cannot be found at start
- **THEN** the engine SHALL emit a `BinaryNotFound` typed error rather than a free-form string

### Requirement: Backpressure

Per-session output buffering SHALL be bounded; when a consumer is slow the engine SHALL apply backpressure (pause/resume the source) or an explicit, logged drop policy, never unbounded buffering.

#### Scenario: Slow consumer

- **WHEN** a consumer reads slower than the agent emits output
- **THEN** the engine SHALL bound memory by pausing the source or dropping per a logged policy

### Requirement: Independent plane degradation

A failure in the status or content plane SHALL NOT terminate the session; the failure SHALL be reported as a typed error while unaffected planes continue.

#### Scenario: Content plane fails, session continues

- **WHEN** structured content cannot be produced for a session
- **THEN** the engine SHALL continue delivering terminal data and status and SHALL report the content failure as a typed error

### Requirement: Resume a prior session

`start({ resume: sessionId })` SHALL relaunch the agent against an existing session id so the conversation continues.

#### Scenario: Resume relaunches with the id

- **WHEN** `start` is called with a `resume` session id
- **THEN** the engine SHALL launch the agent so it continues the identified prior session

### Requirement: Session-correlated observability

The engine SHALL emit structured logs tagged with the session id, and SHALL support an opt-in raw-I/O capture mode for diagnostics that is off by default.

#### Scenario: Logs carry the session id

- **WHEN** the engine logs activity for a session
- **THEN** each log entry SHALL include that session's id

### Requirement: Reconnect to existing session

The engine SHALL expose a `reconnect` operation that reattaches to a session already managed by the daemon, without spawning a new agent process, and returns an `AgentSession` handle with the same event model as a freshly started session.

#### Scenario: Reconnect returns a working session handle

- **WHEN** `reconnect(sessionId, adapter, options)` is called for a session the daemon has live
- **THEN** the engine SHALL return an `AgentSession` that emits data, status, content, and error events identically to a session returned by `start`

#### Scenario: Reconnect delivers replay buffer

- **WHEN** `reconnect` is called for an existing session
- **THEN** the `AgentSession` SHALL emit the replay buffer contents on the data channel before any new data events, so a terminal renderer can restore visual state

#### Scenario: Reconnect to unknown session fails

- **WHEN** `reconnect` is called for a session id the daemon does not have
- **THEN** the engine SHALL reject with a typed error

### Requirement: List daemon sessions

The engine SHALL expose a `listSessions` operation that returns the ids of all sessions currently alive in the daemon, so callers can determine which sessions are reconnectable.

#### Scenario: Returns live ids

- **WHEN** `listSessions()` is called
- **THEN** the engine SHALL return an array of session ids currently registered in the daemon

#### Scenario: Returns empty array when daemon has no sessions

- **WHEN** `listSessions()` is called and the daemon has no active sessions
- **THEN** the engine SHALL return an empty array

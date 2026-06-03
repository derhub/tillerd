# pty-transport

## Purpose

Defines the PTY drive plane: how the engine launches an agent inside a pseudo-terminal, streams raw bytes, handles input/resize, resolves the binary, and tears down the process cleanly.

## Requirements

### Requirement: Clean interactive launch

The PTY drive plane SHALL run the agent inside a pseudo-terminal so the agent behaves as fully interactive, and SHALL launch it so the byte stream contains only the agent's terminal output — no shell prompt or echoed launch command. This SHALL be achieved by launching the agent via the user's login shell environment using exec-replace, so the user's environment is loaded but no shell UI leaks. The launch SHALL be initiated by the daemon on behalf of the engine via the IPC control channel.

#### Scenario: Agent perceives an interactive terminal

- **WHEN** the agent is launched through the PTY drive plane
- **THEN** the agent SHALL run in interactive mode (colors, prompts, live rendering), not headless mode

#### Scenario: No shell noise in the stream

- **WHEN** a session starts
- **THEN** the first bytes delivered SHALL be the agent's own terminal output, with no shell prompt or echoed command preceding it

#### Scenario: Launch delegated to daemon

- **WHEN** the engine requests a new session via the IPC control channel
- **THEN** the daemon SHALL own and manage the resulting PTY master file descriptor for the lifetime of the session

### Requirement: Binary resolution

The drive plane SHALL resolve the agent binary via an explicit override path, then the login-shell PATH, then common install locations; if it cannot be found it SHALL fail with a `BinaryNotFound` error.

#### Scenario: Resolve from the user environment

- **WHEN** the agent binary is installed on the user's interactive PATH
- **THEN** the drive plane SHALL locate it via the login-shell environment and launch it

#### Scenario: Missing binary

- **WHEN** the agent binary cannot be resolved by any method
- **THEN** the drive plane SHALL emit `BinaryNotFound`

### Requirement: Bidirectional raw byte I/O

The drive plane SHALL stream raw bytes out of the PTY and write raw bytes into it without transformation.

#### Scenario: Output streaming

- **WHEN** the agent writes to its terminal
- **THEN** the drive plane SHALL emit the raw bytes with no ANSI stripping or re-decoding

#### Scenario: Input writing

- **WHEN** raw input bytes are forwarded to the session
- **THEN** the drive plane SHALL write those exact bytes to the pseudo-terminal

### Requirement: Interrupt key

The drive plane SHALL cancel the in-progress turn by writing the interrupt key sequence (ESC) to the terminal, without terminating the session.

#### Scenario: Interrupt the current turn

- **WHEN** the current turn is interrupted
- **THEN** the drive plane SHALL write the interrupt key sequence (ESC) to the terminal and the agent SHALL stop the in-progress turn

### Requirement: Terminal resize propagation

The drive plane SHALL resize the underlying pseudo-terminal on request so the agent re-renders for the new dimensions.

#### Scenario: Apply new dimensions

- **WHEN** a resize to given columns and rows is requested
- **THEN** the drive plane SHALL set the pseudo-terminal dimensions accordingly

### Requirement: Process teardown

The drive plane SHALL terminate the agent and reclaim the pseudo-terminal when the session ends, escalating from a graceful stop signal to a forced kill after a grace period.

#### Scenario: Kill releases the terminal

- **WHEN** the session is terminated
- **THEN** the drive plane SHALL terminate the process, free the pseudo-terminal, and report the exit code/signal

### Requirement: PTY sessions survive host process restart

The PTY drive plane SHALL operate inside the daemon process so that active PTY sessions remain alive when the engine's host process exits or restarts. The engine SHALL reconnect to the daemon and resume driving the session without restarting the agent process.

#### Scenario: Session alive after host restart

- **WHEN** the engine host process exits and restarts while a session is running
- **THEN** the agent process SHALL still be running when the engine reconnects to the daemon

#### Scenario: Resume driving after reconnect

- **WHEN** the engine reconnects to the daemon and subscribes to an existing session
- **THEN** the engine SHALL be able to send prompts, receive output, and receive hook events as if no interruption occurred

### Requirement: First-run blocker handling

The drive plane SHALL launch with options that skip first-run blocking dialogs (workspace trust, permission prompts, onboarding) where the agent supports it, and SHALL not hang on them. An agent that never reaches readiness — including a not-logged-in state — SHALL be bounded by the startup timeout and surfaced as a typed startup error rather than left waiting indefinitely.

#### Scenario: Agent never becomes ready

- **WHEN** the agent is launched but does not reach the ready state (for example, the user is not authenticated)
- **THEN** the drive plane SHALL detect the unready state within the startup timeout and emit a typed `Timeout` error, terminating the session

# pty-transport

## Purpose

Defines the PTY drive plane: how the engine launches an agent inside a pseudo-terminal, streams raw bytes, handles input/resize, resolves the binary, and tears down the process cleanly.

## Requirements

### Requirement: Clean interactive launch

The PTY drive plane SHALL run the given command inside a pseudo-terminal so it behaves as fully
interactive. A session SHALL be spawned from a launch config carrying the command, arguments,
working directory, and environment; the drive plane SHALL spawn that command directly, not
through a login-shell wrapper or exec-replace. When no command is supplied the drive plane SHALL
launch the user's login shell. The drive plane SHALL install the login-shell environment at
startup so spawned commands inherit a user-terminal environment. Because the command is spawned
directly rather than via a shell, the byte stream SHALL contain only the spawned process's own
output, with no wrapper shell prompt or echoed launch command. The launch SHALL be initiated by
the daemon on behalf of the engine via the IPC control channel, and the daemon SHALL own and
manage the resulting PTY master file descriptor for the lifetime of the session.

#### Scenario: Spawned process perceives an interactive terminal

- **WHEN** a command is launched through the PTY drive plane
- **THEN** it SHALL run in interactive mode (colors, prompts, live rendering), not headless mode

#### Scenario: Default launch is the login shell

- **WHEN** a session is spawned with no command in the launch config
- **THEN** the drive plane SHALL launch the user's login shell inside the pseudo-terminal

#### Scenario: No wrapper noise in the stream

- **WHEN** a session starts with an explicit command
- **THEN** the first bytes delivered SHALL be that command's own output, with no wrapper shell
  prompt or echoed launch command preceding it

#### Scenario: Launch delegated to daemon

- **WHEN** the engine requests a new session via the IPC control channel
- **THEN** the daemon SHALL own and manage the resulting PTY master file descriptor for the
  lifetime of the session

### Requirement: Binary resolution

The drive plane SHALL resolve the launch command generically: an explicit absolute path SHALL be
used as given; a bare command name SHALL be resolved against the login-shell PATH; when no
command is supplied the login shell SHALL be used. If a named command cannot be resolved the
drive plane SHALL fail with a `BinaryNotFound` error. The drive plane SHALL NOT carry any
application-specific default command, hardcoded install location, or version gate; agent-specific
resolution is the caller's concern.

#### Scenario: Absolute path is used directly

- **WHEN** the launch command is an absolute path to an executable
- **THEN** the drive plane SHALL launch that executable without further resolution

#### Scenario: Resolve a bare name from the user environment

- **WHEN** the launch command is a bare name present on the login-shell PATH
- **THEN** the drive plane SHALL locate it via the login-shell environment and launch it

#### Scenario: Missing command

- **WHEN** a named launch command cannot be resolved by any method
- **THEN** the drive plane SHALL emit `BinaryNotFound`

#### Scenario: No application-specific default

- **WHEN** the launch config supplies no command
- **THEN** the drive plane SHALL default to the login shell and SHALL NOT substitute any
  application-specific default binary or search application-specific install locations

### Requirement: Bidirectional raw byte I/O

The drive plane SHALL stream raw bytes out of the PTY and write raw bytes into it without transformation.

#### Scenario: Output streaming

- **WHEN** the agent writes to its terminal
- **THEN** the drive plane SHALL emit the raw bytes with no ANSI stripping or re-decoding

#### Scenario: Input writing

- **WHEN** raw input bytes are forwarded to the session
- **THEN** the drive plane SHALL write those exact bytes to the pseudo-terminal

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

### Requirement: Generic launch-config environment

The launch config SHALL carry the spawned process's environment as a generic string map rather
than as discrete application-named fields. The drive plane SHALL compute the child environment as
its generic terminal base environment (a standard terminal allowlist such as PATH, HOME, USER,
SHELL, LANG, TERM, and COLORTERM, derived from the login-shell environment installed at startup)
merged with the caller-supplied environment map, with caller entries taking precedence. The drive
plane SHALL NOT inject or reference any application-specific environment variable by name; any
such variable SHALL be provided by the caller within the environment map.

#### Scenario: Caller environment is merged over the terminal base

- **WHEN** a session is spawned with an environment map
- **THEN** the spawned process SHALL receive the terminal base environment with the caller's
  entries merged on top, the caller's values winning on conflict

#### Scenario: No application-specific variable is named by the drive plane

- **WHEN** a session is spawned
- **THEN** the drive plane SHALL NOT add any application-specific environment variable of its own;
  variables such as a hook-socket address or session token SHALL appear only if the caller placed
  them in the environment map

#### Scenario: Terminal base is always present

- **WHEN** a session is spawned with an empty or partial environment map
- **THEN** the spawned process SHALL still receive the generic terminal base environment so it
  behaves as an interactive terminal

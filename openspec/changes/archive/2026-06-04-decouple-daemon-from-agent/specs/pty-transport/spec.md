## MODIFIED Requirements

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

## ADDED Requirements

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

## REMOVED Requirements

### Requirement: Interrupt key

**Reason**: Cancelling a turn is agent policy, not a drive-plane concern. The drive plane now
forwards input bytes verbatim and assigns no special meaning to any key; the engine cancels a
turn by writing the adapter-defined interrupt sequence through the normal raw-input path.

**Migration**: The adapter (`AgentDefinition`) supplies an interrupt-sequence datum; the engine
writes those bytes via the existing raw-input channel to cancel the current turn. The daemon's
dedicated interrupt command is removed.

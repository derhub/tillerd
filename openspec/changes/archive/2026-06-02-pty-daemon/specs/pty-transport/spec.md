## ADDED Requirements

### Requirement: PTY sessions survive host process restart

The PTY drive plane SHALL operate inside the daemon process so that active PTY sessions remain alive when the engine's host process exits or restarts. The engine SHALL reconnect to the daemon and resume driving the session without restarting the agent process.

#### Scenario: Session alive after host restart

- **WHEN** the engine host process exits and restarts while a session is running
- **THEN** the agent process SHALL still be running when the engine reconnects to the daemon

#### Scenario: Resume driving after reconnect

- **WHEN** the engine reconnects to the daemon and subscribes to an existing session
- **THEN** the engine SHALL be able to send prompts, receive output, and receive hook events as if no interruption occurred

## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Clean interactive launch

The PTY drive plane SHALL run the agent inside a pseudo-terminal so the agent behaves as fully interactive, and SHALL launch it so the byte stream contains only the agent's terminal output — no shell prompt or echoed launch command. This SHALL be achieved by spawning the user's login shell and replacing it with the agent process (`exec`), so the user's environment is loaded but no shell UI leaks.

#### Scenario: Agent perceives an interactive terminal

- **WHEN** the agent is launched through the PTY drive plane
- **THEN** the agent SHALL run in interactive mode (colors, prompts, live rendering), not headless mode

#### Scenario: No shell noise in the stream

- **WHEN** a session starts
- **THEN** the first bytes delivered SHALL be the agent's own terminal output, with no shell prompt or echoed command preceding it

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

### Requirement: Prompt submission and interrupt keys

The drive plane SHALL submit a prompt turn by writing the prompt text using bracketed paste (so multi-line input is delivered intact) followed by the submit key, and SHALL interrupt the current turn by writing the agent's interrupt key.

#### Scenario: Multi-line prompt submitted intact

- **WHEN** a multi-line prompt is submitted
- **THEN** the drive plane SHALL deliver it via bracketed paste and a single submit, not as separate line-by-line submissions

#### Scenario: Interrupt the current turn

- **WHEN** the current turn is interrupted
- **THEN** the drive plane SHALL write the agent's interrupt key and the agent SHALL stop the in-progress turn

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

### Requirement: First-run blocker handling

The drive plane SHALL launch with options that skip first-run blocking dialogs (workspace trust, onboarding) where the agent supports it, and SHALL not hang on them; a not-logged-in state SHALL be surfaced as `NotAuthenticated` rather than left waiting on a login prompt.

#### Scenario: Not logged in

- **WHEN** the agent is launched but the user is not authenticated
- **THEN** the drive plane SHALL detect the unready state within the startup timeout and emit `NotAuthenticated`

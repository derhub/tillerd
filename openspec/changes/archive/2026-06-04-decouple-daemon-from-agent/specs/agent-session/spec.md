## MODIFIED Requirements

### Requirement: Interrupt versus kill

`interrupt` SHALL cancel the current turn while keeping the session alive; `kill` SHALL
terminate the session. The engine SHALL cancel the current turn by writing the adapter-supplied
interrupt sequence through the raw-input path, not via a dedicated daemon interrupt command.

#### Scenario: Interrupt keeps the session

- **WHEN** `interrupt()` is called during a WORKING turn
- **THEN** the engine SHALL write the adapter's interrupt sequence through the raw-input path, the
  in-progress turn SHALL be cancelled, and the session SHALL remain usable for further prompts

#### Scenario: Kill terminates the session

- **WHEN** `kill()` is called
- **THEN** the engine SHALL terminate the session

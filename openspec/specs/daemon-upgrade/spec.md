# daemon-upgrade

## Purpose

Defines the drain-and-restart upgrade protocol for the daemon: on a planned upgrade the daemon drains active sessions then exits cleanly, and the supervisor starts the new binary fresh. Session continuity comes from workspace persistence and the agent CLI's own resume, not from process handoff.

## Requirements

### Requirement: Drain-and-restart upgrade

On a planned upgrade the daemon SHALL drain: refuse new sessions, let active sessions
finish, then exit cleanly. The supervisor SHALL then start the new binary fresh. Session
continuity across the upgrade SHALL come from workspace persistence plus the agent CLI's
own resume, not from process handoff.

#### Scenario: Draining daemon refuses new sessions

- **WHEN** the daemon is draining and a client requests a new session
- **THEN** the request is refused with a typed error and active sessions are unaffected

#### Scenario: Daemon exits once active sessions finish

- **WHEN** the last active session on a draining daemon ends
- **THEN** the daemon exits cleanly and removes its manifest

#### Scenario: Explicit upgrade-now path

- **WHEN** the user chooses to upgrade immediately while sessions are active
- **THEN** active sessions are terminated deliberately and the daemon exits cleanly;
  no timer kills sessions automatically

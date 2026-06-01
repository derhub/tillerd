# agent-status

## Purpose

Defines how the engine maps agent lifecycle events to the fixed status contract `{ IDLE | WORKING | WAITING_INPUT | DONE }`. The mapper is generic — it depends only on `HookEvent` contract types, never on adapter-specific payload shapes or transport details.

## Requirements

### Requirement: Generic contract-enum to status mapping

The status mapper SHALL map a `HookEvent`'s contract type to the fixed status set `{ IDLE | WORKING | WAITING_INPUT | DONE }` using a single generic mapping, with no per-adapter table. It SHALL consume only `HookEvent` values and SHALL have no knowledge of transport, HTTP, or raw payload shapes.

#### Scenario: Session start maps to idle

- **WHEN** a `HookEvent` of type SessionStart is dispatched
- **THEN** the session status SHALL become IDLE

#### Scenario: Prompt or tool activity maps to working

- **WHEN** a `HookEvent` of type UserPromptSubmit or PostToolUse is dispatched
- **THEN** the session status SHALL become WORKING

#### Scenario: Permission or input request maps to waiting

- **WHEN** a `HookEvent` of type PermissionRequest is dispatched
- **THEN** the session status SHALL become WAITING_INPUT

#### Scenario: Turn stop maps back to idle

- **WHEN** a `HookEvent` of type Stop is dispatched
- **THEN** the session status SHALL become IDLE

#### Scenario: Session end maps to done

- **WHEN** a `HookEvent` of type SessionEnd is dispatched
- **THEN** the session status SHALL become DONE

### Requirement: IDLE means ready for input

IDLE SHALL denote "awaiting the user" — whether the agent has finished or has ended its turn with a question — and SHALL be the state at which the session is ready to accept the next prompt.

#### Scenario: Question ends the turn as idle

- **WHEN** the agent ends a turn by asking the user a question (ending the turn)
- **THEN** the status SHALL be IDLE and the session SHALL accept the next prompt

### Requirement: Idempotent status application

Applying the same `HookEvent` more than once SHALL leave the status unchanged from a single application.

#### Scenario: Duplicate event

- **WHEN** the same status-affecting `HookEvent` is applied twice
- **THEN** the resulting status SHALL equal the result of applying it once

### Requirement: Status changes are emitted

Each status transition SHALL be emitted on the session's status channel.

#### Scenario: Transition emitted

- **WHEN** a session moves from WORKING to IDLE
- **THEN** a status event reflecting IDLE SHALL be emitted to consumers

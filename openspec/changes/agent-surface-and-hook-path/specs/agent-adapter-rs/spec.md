## ADDED Requirements

### Requirement: AgentDefinition structure in the orchestrator

The orchestrator SHALL define an `AgentDefinition` data structure capturing: the agent binary name, launch arguments template (with a `{surface_id}` placeholder), an optional CLI version range, an interrupt byte sequence, and a binary-resolution policy. This structure is library-free and carries no runtime state.

#### Scenario: Definition is read by the surface runtime before spawn

- **WHEN** the orchestrator opens an agent surface
- **THEN** it SHALL read the launch args from the `AgentDefinition` bound to that surface kind, substituting `{surface_id}` with the concrete surface id before passing the command to the process launcher

#### Scenario: Binary resolution produces a concrete path

- **WHEN** the orchestrator resolves the agent binary
- **THEN** it SHALL use the resolution policy from the `AgentDefinition` (PATH lookup, version check) and return a typed error if the binary is absent or its version is outside the declared range

### Requirement: Hook-event to agent-status mapping

The orchestrator SHALL provide a pure, deterministic function that maps a `HookEvent` variant to an `AgentStatus` variant (`IDLE`, `WORKING`, `WAITING_INPUT`, `DONE`). The function SHALL have no side effects and SHALL accept any `HookEvent` without panicking.

#### Scenario: SessionStart maps to IDLE

- **WHEN** a `SessionStart` hook event is mapped
- **THEN** the function SHALL return `AgentStatus::Idle`

#### Scenario: UserPromptSubmit maps to WORKING

- **WHEN** a `UserPromptSubmit` hook event is mapped
- **THEN** the function SHALL return `AgentStatus::Working`

#### Scenario: PostToolUse maps to WORKING

- **WHEN** a `PostToolUse` hook event is mapped
- **THEN** the function SHALL return `AgentStatus::Working`

#### Scenario: PermissionRequest maps to WAITING_INPUT

- **WHEN** a `PermissionRequest` hook event is mapped
- **THEN** the function SHALL return `AgentStatus::WaitingInput`

#### Scenario: Stop maps to IDLE

- **WHEN** a `Stop` hook event is mapped
- **THEN** the function SHALL return `AgentStatus::Idle`

#### Scenario: SessionEnd maps to DONE

- **WHEN** a `SessionEnd` hook event is mapped
- **THEN** the function SHALL return `AgentStatus::Done`

### Requirement: Hook-event to content-event mapping

The orchestrator SHALL provide a pure function that maps a `HookEvent` variant to an optional `ContentEvent`. The function SHALL return `None` for event variants that carry no displayable content.

#### Scenario: PostToolUse produces a tool-use content event

- **WHEN** a `PostToolUse` hook event is mapped
- **THEN** the function SHALL return a `ContentEvent` of kind `tool_use` carrying the tool name and tool input

#### Scenario: Other event variants produce no content event

- **WHEN** any hook event other than `PostToolUse` is mapped
- **THEN** the function SHALL return `None`

### Requirement: Typed AgentStatus enum in contracts

The `contracts` crate SHALL define `AgentStatus` as a closed enum with variants `Idle`, `Working`, `WaitingInput`, `Done`. It SHALL be serializable, comparable for equality, and copyable.

#### Scenario: AgentStatus round-trips through the wire protocol

- **WHEN** an `AgentStatus` value is serialized and then deserialized
- **THEN** the result SHALL equal the original value

# app-use-case-layer Specification

## Purpose
TBD - created by archiving change app-use-case-layer. Update Purpose after archive.
## Requirements
### Requirement: Session creation resolves a launch template

The app layer SHALL own session creation: given a draft, it resolves the draft's launch
template (if any) into a concrete spec and materializes the session. This coordination spans the
`LaunchTemplates` and `Sessions` aggregates and SHALL NOT live in a store method or a host
controller.

#### Scenario: Draft with a template id materializes a session carrying the instantiated spec

- **WHEN** `create_session` is called with a draft whose `template_id` references an existing template
- **THEN** the template is resolved, its spec is instantiated for the session, and the persisted session carries that spec at the template's spec version

#### Scenario: Draft without a template id materializes a session with no spec

- **WHEN** `create_session` is called with a draft whose `template_id` is `None`
- **THEN** the session is persisted with no launch spec and no template lookup occurs

#### Scenario: Draft referencing an unknown template fails

- **WHEN** `create_session` is called with a `template_id` that does not exist
- **THEN** it returns a `LaunchTemplateNotFound` error and no session is persisted

### Requirement: Opening a session creates then activates it

The app layer SHALL expose an `open_session` use case that sequences create-then-activate:
materialize the session, then activate its surfaces best-effort through a narrow activation port.
Activation is decoupled from the concrete surface runtime so the use case is host-agnostic.

#### Scenario: open_session persists the session and activates its surfaces

- **WHEN** `open_session` is called with a draft and an activator
- **THEN** the session is persisted and the activator is invoked once with the new session's id

#### Scenario: Activation failure is non-fatal

- **WHEN** `open_session` is called and the activator returns an error
- **THEN** the failure is logged and the created session is still returned successfully

### Requirement: Hosts delegate cross-aggregate coordination to the app layer

Host controllers SHALL be pure IPC shims: they map their transport request into a draft and
delegate to the app use case, never assembling the create-then-activate sequence themselves.

#### Scenario: The desktop session-create command delegates to the app use case

- **WHEN** the tauri `session_create` command runs
- **THEN** it builds a draft from its arguments, calls `open_session`, and returns the created session — the command contains no cross-aggregate sequencing of its own


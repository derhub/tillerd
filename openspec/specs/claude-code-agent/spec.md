# claude-code-agent

## Purpose

Defines the `AgentDefinition` adapter contract and the `claudeCode` adapter that implements it. The contract keeps the engine agent-blind; all agent-specific behavior flows through config data and parse functions supplied by the adapter.

## Requirements

### Requirement: Hybrid AgentDefinition contract

The SDK SHALL define an `AgentDefinition` contract that an adapter implements as declarative
config data plus a small set of parse functions, so the engine stays agent-blind:

- config data: a launch template (command, args with placeholders, flags), an interrupt-sequence
  datum (the raw bytes that cancel an in-progress turn), a hook-install spec (settings path,
  command template, event list), and a supported CLI version range;
- functions: `parseHook(raw) -> HookEvent`, `transcriptPath(sessionId, cwd) -> path`, and
  `parseTranscriptEntry(line) -> content`.

#### Scenario: Definition supplies config and functions

- **WHEN** an adapter is provided to the engine
- **THEN** it SHALL expose the launch/interrupt/hook-install/version config as data and the parse
  operations as functions

#### Scenario: Engine stays agent-blind

- **WHEN** the engine processes a session
- **THEN** it SHALL obtain all agent-specific behavior through the adapter's config and functions,
  never hard-coding the agent's payload, transcript, path shapes, or interrupt sequence

### Requirement: Claude Code adapter

The SDK SHALL ship a `claudeCode` adapter implementing the `AgentDefinition` contract for the
target agent. The adapter SHALL own resolution of the agent binary — an explicit override path,
then common install locations — and SHALL supply a launchable command to the engine; the daemon
SHALL NOT default to or search for the agent binary. The adapter SHALL also supply the
interrupt-sequence datum used to cancel a turn.

#### Scenario: Launch without credentials

- **WHEN** `claudeCode` starts a session
- **THEN** it SHALL launch the installed agent binary using the user's existing login, with no
  API key supplied

#### Scenario: Adapter resolves the agent binary

- **WHEN** the engine prepares a launch from the `claudeCode` config
- **THEN** the adapter SHALL resolve the agent binary via an explicit override path then common
  install locations, and SHALL pass the resolved launchable command to the daemon, which spawns
  it without applying any agent-specific resolution of its own

#### Scenario: Adapter supplies the interrupt sequence

- **WHEN** the engine cancels an in-progress turn
- **THEN** it SHALL write the adapter's interrupt-sequence bytes through the raw-input path, and
  the adapter SHALL be the source of those bytes

#### Scenario: Caller-chosen session id

- **WHEN** the engine generates a session id
- **THEN** the `claudeCode` launch config SHALL pass it via `--session-id` so the agent adopts it as its own id

#### Scenario: Permissions punted

- **WHEN** `claudeCode` launches the agent
- **THEN** it SHALL pass `--dangerously-skip-permissions` so the agent does not block on permission prompts

#### Scenario: Hook-install config

- **WHEN** the engine installs hooks from the `claudeCode` config
- **THEN** the config SHALL target `~/.claude/settings.json` and register the events SessionStart, UserPromptSubmit, PostToolUse, PermissionRequest, Stop, and SessionEnd

#### Scenario: Parse hook payloads to contract events

- **WHEN** `parseHook` receives a hook payload
- **THEN** it SHALL return a `HookEvent` whose type is the corresponding contract enum value and whose `sessionId` is extracted from the payload

#### Scenario: Resolve transcript path

- **WHEN** `transcriptPath` is called with a session id and working directory
- **THEN** it SHALL return the path under `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl`, applying the agent's directory-encoding rule

#### Scenario: Parse transcript entries to content

- **WHEN** `parseTranscriptEntry` receives a transcript line describing a tool call, edit, or usage record
- **THEN** it SHALL return the corresponding typed content value (or nothing for lines that carry no content)

### Requirement: Supported CLI version range

The `claudeCode` adapter SHALL declare a supported agent version range, and the engine SHALL
detect the installed version and emit `VersionUnsupported` on mismatch. Version detection SHALL
be performed by the engine from adapter config; the daemon SHALL NOT perform any version gate.

#### Scenario: Unsupported version

- **WHEN** the installed agent version falls outside the adapter's declared range
- **THEN** the engine SHALL emit `VersionUnsupported` rather than silently risk broken
  hook/transcript parsing

#### Scenario: Daemon performs no version gate

- **WHEN** a session is spawned through the daemon
- **THEN** the daemon SHALL NOT detect or gate on any agent version; that responsibility SHALL
  rest with the engine using adapter config

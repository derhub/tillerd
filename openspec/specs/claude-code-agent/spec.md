# claude-code-agent

## Purpose

Defines the `AgentDefinition` adapter contract and the `claudeCode` adapter that implements it. The contract keeps the engine agent-blind; all agent-specific behavior flows through config data and parse functions supplied by the adapter.

## Requirements

### Requirement: Hybrid AgentDefinition contract

The SDK SHALL define an `AgentDefinition` contract that an adapter implements as declarative config data plus a small set of parse functions, so the engine stays agent-blind:

- config data: a launch template (command, args with placeholders, flags), a hook-install spec (settings path, command template, event list), and a supported CLI version range;
- functions: `parseHook(raw) -> HookEvent`, `transcriptPath(sessionId, cwd) -> path`, and `parseTranscriptEntry(line) -> content`.

#### Scenario: Definition supplies config and functions

- **WHEN** an adapter is provided to the engine
- **THEN** it SHALL expose the launch/hook-install/version config as data and the parse operations as functions

#### Scenario: Engine stays agent-blind

- **WHEN** the engine processes a session
- **THEN** it SHALL obtain all agent-specific behavior through the adapter's config and functions, never hard-coding the agent's payload, transcript, or path shapes

### Requirement: Claude Code adapter

The SDK SHALL ship a `claudeCode` adapter implementing the `AgentDefinition` contract for Claude Code.

#### Scenario: Launch without credentials

- **WHEN** `claudeCode` starts a session
- **THEN** it SHALL launch the installed `claude` binary using the user's existing login, with no API key supplied

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

- **WHEN** `parseHook` receives a Claude Code hook payload
- **THEN** it SHALL return a `HookEvent` whose type is the corresponding contract enum value and whose `sessionId` is extracted from the payload

#### Scenario: Resolve transcript path

- **WHEN** `transcriptPath` is called with a session id and working directory
- **THEN** it SHALL return the path under `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl`, applying Claude Code's directory-encoding rule

#### Scenario: Parse transcript entries to content

- **WHEN** `parseTranscriptEntry` receives a transcript line describing a tool call, edit, or usage record
- **THEN** it SHALL return the corresponding typed content value (or nothing for lines that carry no content)

### Requirement: Supported CLI version range

The `claudeCode` adapter SHALL declare a supported `claude` version range, and the engine SHALL detect the installed version and emit `VersionUnsupported` on mismatch.

#### Scenario: Unsupported version

- **WHEN** the installed `claude` version falls outside the adapter's declared range
- **THEN** the engine SHALL emit `VersionUnsupported` rather than silently risk broken hook/transcript parsing

## MODIFIED Requirements

### Requirement: Hybrid AgentDefinition contract

The SDK SHALL define an `AgentDefinition` contract that an adapter implements as declarative
config data plus a small set of pure functions, so the engine stays agent-blind and the adapter
performs no host I/O:

- config data: a launch template (command, args with placeholders, flags), an interrupt-sequence
  datum (the raw bytes that cancel an in-progress turn), a hook-install spec (settings-file
  location, command template, event list, hook marker, and per-event matcher rule), and a
  supported CLI version range;
- functions: `parseHook(raw) -> HookEvent`, `parseTranscriptEntry(line) -> content`,
  `transcriptPath(sessionId, cwd, agentHome) -> path`, and pure hook-planning functions that,
  given the current settings value and a notify command, compute the next settings value and
  what changed — `planHookInstall(currentSettings, notifyCommand)` and
  `planHookUninstall(currentSettings)`.

The contract SHALL NOT include any method that reads, writes, or otherwise touches the
filesystem; performing the settings file read, backup, and write is the host's responsibility.

#### Scenario: Definition supplies config and functions

- **WHEN** an adapter is provided to the engine
- **THEN** it SHALL expose the launch/interrupt/hook-install/version config as data and the parse
  and hook-planning operations as pure functions

#### Scenario: Engine stays agent-blind

- **WHEN** the engine processes a session
- **THEN** it SHALL obtain all agent-specific behavior through the adapter's config and functions,
  never hard-coding the agent's payload, transcript, path shapes, or interrupt sequence

#### Scenario: Hook planning is a pure transform

- **WHEN** the host installs or removes hooks
- **THEN** it SHALL obtain the next settings value from the adapter's hook-planning function given
  the current settings value, and the adapter SHALL neither read nor write the settings file
- **AND** when the requested hooks are already present (or already absent for removal), the plan
  SHALL report no change

### Requirement: Claude Code adapter

The SDK SHALL ship a `claudeCode` adapter implementing the `AgentDefinition` contract for the
target agent. The adapter SHALL own the binary-resolution policy — an override environment
variable, the binary name, and common install locations — as declarative data; the host SHALL
perform the lookup I/O and supply the resolved launchable command to the engine as a
startup-resolved value; the daemon SHALL NOT default to or search for the agent binary. The
adapter SHALL also supply the interrupt-sequence datum used to cancel a turn.

#### Scenario: Launch without credentials

- **WHEN** `claudeCode` starts a session
- **THEN** it SHALL launch the installed agent binary using the user's existing login, with no
  API key supplied

#### Scenario: Adapter owns the binary-resolution policy, host performs the lookup

- **WHEN** the host prepares a launch from the `claudeCode` config
- **THEN** the adapter SHALL supply the resolution policy (override env var, binary name, common
  install locations) as declarative data, the host SHALL perform the lookup — override path, then
  login-shell PATH, then common locations — and SHALL pass the resolved launchable command to the
  engine as a startup value, which the daemon spawns without applying any agent-specific
  resolution of its own
- **AND** the adapter SHALL expose no method that reads the filesystem, environment, or PATH

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

- **WHEN** the host installs hooks from the `claudeCode` config
- **THEN** the config SHALL target the agent settings file at `~/.claude/settings.json` and register
  the events SessionStart, UserPromptSubmit, PostToolUse, PermissionRequest, Stop, and SessionEnd
- **AND** the host SHALL perform the read, backup, and write of that file using the adapter's
  declarative spec and pure plan

#### Scenario: Parse hook payloads to contract events

- **WHEN** `parseHook` receives a hook payload
- **THEN** it SHALL return a `HookEvent` whose type is the corresponding contract enum value and whose `sessionId` is extracted from the payload

#### Scenario: Resolve transcript path

- **WHEN** `transcriptPath` is called with a session id, working directory, and an agent-home input
- **THEN** it SHALL return the path under `<agentHome>/projects/<encoded-cwd>/<session-id>.jsonl`,
  applying the agent's directory-encoding rule using pure string operations and reading no ambient
  home or path host primitive

#### Scenario: Parse transcript entries to content

- **WHEN** `parseTranscriptEntry` receives a transcript line describing a tool call, edit, or usage record
- **THEN** it SHALL return the corresponding typed content value (or nothing for lines that carry no content)

## ADDED Requirements

### Requirement: Adapter module is import-safe in any runtime

The adapter module SHALL be importable in any runtime, including a browser-class web view,
without accessing host primitives at module load. It SHALL NOT, at import time or within any of
its contract functions, read a filesystem, read an ambient home directory, environment, or
current-directory global, or otherwise depend on a host-specific runtime capability.

#### Scenario: Importing the adapter touches no host primitive

- **WHEN** a host imports the adapter and reads its config or invokes its pure functions
- **THEN** no filesystem access and no ambient host-global access SHALL occur as a result
- **AND** a renderer-class host SHALL be able to import the adapter and hand it to the engine to
  drive a session

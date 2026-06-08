# claude-code-agent

## Purpose

Defines the `AgentDefinition` adapter contract and the `claudeCode` adapter that implements it. The contract keeps the engine agent-blind; all agent-specific behavior flows through config data and parse functions supplied by the adapter.

## Requirements

### Requirement: Hybrid AgentDefinition contract

The SDK SHALL define an `AgentDefinition` contract that an adapter implements as declarative config
data plus a small set of pure parse functions, so the engine stays agent-blind and the
engine-facing definition performs no host I/O:

- config data: a launch template (command, args with placeholders, flags), an interrupt-sequence
  datum (the raw bytes that cancel an in-progress turn), a binary-resolution policy (override
  environment variable, binary name, common install locations), and a supported CLI version range;
- functions: `parseHook(raw) -> HookEvent`, `parseTranscriptEntry(line) -> content`, and
  `transcriptPath(sessionId, cwd, agentHome) -> path`.

The engine-facing definition SHALL NOT include any setup, install, or other host-I/O member; the
adapter SHALL supply its host setup separately through the adapter-setup contract.

#### Scenario: Definition supplies config and functions

- **WHEN** an adapter is provided to the engine
- **THEN** it SHALL expose the launch/interrupt/binary-resolution/version config as data and the
  parse and path operations as pure functions

#### Scenario: Engine stays agent-blind

- **WHEN** the engine processes a session
- **THEN** it SHALL obtain all agent-specific behavior through the adapter's config and functions,
  never hard-coding the agent's payload, transcript, path shapes, or interrupt sequence

#### Scenario: Setup is not part of the engine-facing definition

- **WHEN** a host imports the engine-facing adapter definition
- **THEN** that definition SHALL expose no setup, install, or uninstall member, and SHALL touch no
  filesystem or ambient host primitive; setup is reached only through the separate adapter-setup
  contract

### Requirement: Claude Code adapter

The SDK SHALL ship a `claudeCode` adapter implementing the `AgentDefinition` contract for the
target agent. The adapter SHALL own the binary-resolution policy — an override environment
variable, the binary name, and common install locations — as declarative data; the host SHALL
perform the lookup I/O and supply the resolved launchable command to the engine as a
startup-resolved value; the daemon SHALL NOT default to or search for the agent binary. The
adapter SHALL also supply the interrupt-sequence datum used to cancel a turn, and SHALL supply its
host setup through the adapter-setup contract.

#### Scenario: Launch without credentials

- **WHEN** `claudeCode` starts a session
- **THEN** it SHALL launch the installed agent binary using the user's existing login, with no
  API key supplied

#### Scenario: Adapter owns the binary-resolution policy, host performs the lookup

- **WHEN** the host prepares a launch from the `claudeCode` config
- **THEN** the adapter SHALL supply the resolution policy (override env var, binary name, common
  install locations) as declarative data, the host SHALL perform the lookup and pass the resolved
  launchable command to the engine as a startup value, and the daemon SHALL spawn it without
  applying any agent-specific resolution of its own

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

#### Scenario: Setup installs the hook integration

- **WHEN** the host invokes the adapter's setup `install`
- **THEN** the adapter SHALL register the notify hook in the agent settings file at
  `<agentHome>/settings.json` for the events SessionStart, UserPromptSubmit, PostToolUse,
  PermissionRequest, Stop, and SessionEnd, reading and persisting the file through the host
  filesystem capability rather than touching the filesystem directly

#### Scenario: Parse hook payloads to contract events

- **WHEN** `parseHook` receives a hook payload
- **THEN** it SHALL return a `HookEvent` whose type is the corresponding contract enum value and whose `sessionId` is extracted from the payload

#### Scenario: Resolve transcript path

- **WHEN** `transcriptPath` is called with a session id, working directory, and an agent-home input
- **THEN** it SHALL return the path under `<agentHome>/projects/<encoded-cwd>/<session-id>.jsonl`,
  applying the agent's directory-encoding rule using pure string operations

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

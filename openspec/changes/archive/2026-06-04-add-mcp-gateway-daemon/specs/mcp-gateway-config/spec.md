## ADDED Requirements

### Requirement: Config file location and resolution

The gateway SHALL read its backend configuration from a file named `mcp.json` in the application
data directory. The data directory SHALL be resolved from the `ATHING_DIR` environment variable
when set and non-empty (relative values resolved against the current working directory), otherwise
from `~/.athing`. A missing config file SHALL NOT be an error: the gateway SHALL start with zero
backends.

#### Scenario: Default location

- **WHEN** `ATHING_DIR` is unset
- **THEN** the gateway SHALL read `~/.athing/mcp.json`

#### Scenario: Overridden location

- **WHEN** `ATHING_DIR` is set to an absolute directory path
- **THEN** the gateway SHALL read `mcp.json` from that directory

#### Scenario: Missing file starts empty

- **WHEN** the config file does not exist
- **THEN** the gateway SHALL start successfully with no backends and SHALL NOT report an error

### Requirement: Backend server format

The config SHALL declare backends under an `mcpServers` object keyed by a unique backend name. A
backend SHALL be either a process backend, carrying a `command` with optional `args` and `env`, or
a remote backend, carrying a `url` with optional `headers`. The presence of `command` SHALL select
a process backend and the presence of `url` SHALL select a remote backend. The backend name SHALL
be used as the namespace prefix for that backend's tools, resources, and prompts.

#### Scenario: Process backend

- **WHEN** a backend entry has a `command` and `args`
- **THEN** the gateway SHALL treat it as a process backend to be spawned with those arguments and
  environment

#### Scenario: Remote backend

- **WHEN** a backend entry has a `url`
- **THEN** the gateway SHALL treat it as a remote backend reached at that URL with the given headers

#### Scenario: Names are unique namespaces

- **WHEN** two backends are declared
- **THEN** each backend's name SHALL be distinct and SHALL serve as the prefix for its exposed
  primitives

### Requirement: Per-backend allowlist and lazy extensions

A backend entry SHALL accept an optional `allowedTools` array and an optional `lazy` boolean. When
`allowedTools` is present, only the listed tool names SHALL be exposed for that backend; when absent,
all of the backend's tools SHALL be exposed. The `lazy` flag SHALL default to `false`.

#### Scenario: Allowlist filters tools

- **WHEN** a backend declares `allowedTools` with a subset of its tool names
- **THEN** the gateway SHALL expose only those tools for that backend and SHALL reject calls to its
  other tools

#### Scenario: Omitted allowlist exposes all

- **WHEN** a backend omits `allowedTools`
- **THEN** the gateway SHALL expose all of that backend's tools

#### Scenario: Lazy defaults off

- **WHEN** a backend omits `lazy`
- **THEN** the gateway SHALL treat the backend as not lazy

### Requirement: Loose parsing for paste compatibility

The gateway SHALL accept unknown keys within a backend entry without failing, so that configurations
authored for other MCP clients can be used unchanged, and SHALL record each unknown backend key for
diagnostics. The top-level object SHALL permit only `mcpServers` and `$schema`; any other top-level
key SHALL be rejected with a typed error that names the offending key.

#### Scenario: Unknown backend key tolerated

- **WHEN** a backend entry contains a key the gateway does not recognize
- **THEN** the gateway SHALL load the backend successfully and SHALL log the unknown key

#### Scenario: Unknown top-level key rejected

- **WHEN** the config contains a top-level key other than `mcpServers` or `$schema`
- **THEN** the gateway SHALL fail to load with a typed error naming that key

#### Scenario: Schema pointer permitted

- **WHEN** the config contains a `$schema` string at the top level
- **THEN** the gateway SHALL accept it and ignore it for behavior

### Requirement: Published schema with drift protection

The gateway SHALL ship a `schema.json` describing the config contract, generated from the same type
definitions that parse the config. A test SHALL fail when the published `schema.json` does not match
the schema generated from the current types. The sample config SHALL include a `$schema` pointer.

#### Scenario: Schema matches types

- **WHEN** the config types change without regenerating `schema.json`
- **THEN** the drift test SHALL fail

#### Scenario: Invalid config reports field context

- **WHEN** a backend value has the wrong type
- **THEN** the gateway SHALL fail to load with a typed error identifying the backend and field

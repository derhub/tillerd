## MODIFIED Requirements

### Requirement: Host supplies startup-resolved values

The engine SHALL receive the values produced by one-time startup resolution — a connected
daemon transport, a file-read contract, a logger, the working directory, the resolved agent
invocation, the agent-home location, and the hook callback configuration — from its caller, and
SHALL NOT perform process spawning, executable resolution, version probing, or home-directory
resolution itself.

#### Scenario: Starting a session performs no in-engine environment probing

- **WHEN** a session is started
- **THEN** the engine uses the caller-supplied resolved values, including the working directory
  and the agent-home location
- **AND** it does not spawn a supervisory process, resolve an executable path, probe a tool
  version, or resolve a home directory on its own

#### Scenario: Engine threads the agent-home value into transcript resolution

- **WHEN** the engine resolves the transcript path for a session through the adapter
- **THEN** it SHALL supply the caller-resolved agent-home location to the adapter's transcript-path
  function rather than reading an ambient home or path host primitive

#### Scenario: Engine spawns with the caller-resolved agent invocation

- **WHEN** the engine launches an agent session
- **THEN** it SHALL use the caller-resolved agent command supplied at startup as the spawn command,
  and SHALL NOT call any adapter method or host primitive to resolve the executable itself

## MODIFIED Requirements

### Requirement: Surface creation dispatches by kind

The "which kinds may spawn" capability rule SHALL be enforced in the app surface-spawn use case, before any persistence or runtime effect. In 0.x the only runnable kind is `terminal`; a request for a kind with no launch adapter (e.g. `diff`) SHALL be rejected with a typed validation error and SHALL create neither a surface row nor a runtime proxy. The surface runtime SHALL be kind-agnostic: given a spawn request it spawns a pseudo-terminal and yields the per-surface proxy it owns, without inspecting the surface kind.

#### Scenario: Terminal kind spawns and yields a proxy

- **WHEN** a terminal surface is created
- **THEN** the app handler persists the pending row and the runtime spawns the command and returns the proxy it stores

#### Scenario: An unsupported kind is rejected before any effect

- **WHEN** a surface of a kind with no launch adapter (e.g. `diff`) is requested
- **THEN** the app handler returns a typed validation error and no surface row and no proxy are created

#### Scenario: The runtime does not inspect kind

- **WHEN** the runtime receives a spawn request
- **THEN** it spawns a pseudo-terminal for the surface without branching on kind, and rejects only a duplicate proxy for the same surface as raw resource integrity

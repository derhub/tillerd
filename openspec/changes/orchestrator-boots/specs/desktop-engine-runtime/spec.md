## REMOVED Requirements

### Requirement: Agent engine runs inside the renderer

**Reason**: The backend moves to the embedded runtime-agnostic orchestrator (ADR-0022); the
renderer no longer hosts the agent engine or adapter.
**Migration**: Backend interaction goes through the SDK orchestrator-API client (capability
`sdk-orchestrator-client`); the engine and adapter run in the orchestrator (capability
`orchestrator-core`). The renderer reaches a usable state by observing the orchestrator `ready`
state through the SDK client.

### Requirement: Engine constructed from native startup values

**Reason**: The engine is no longer constructed in the renderer; the embedded orchestrator owns
its own construction in-process from host-resolved values.
**Migration**: The host embeds the orchestrator, which resolves its startup values and boots to
`ready`; the renderer consumes only the orchestrator API through the SDK client and no longer
constructs an engine or supplies engine startup values.

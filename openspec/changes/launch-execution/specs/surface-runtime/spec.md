# Surface Runtime

## ADDED Requirements

### Requirement: Surface creation dispatches to a kind adapter

The surface runtime SHALL create a surface by selecting an adapter for the surface's kind and
delegating to it; the runtime SHALL NOT branch on kind beyond adapter selection. Each adapter SHALL
produce the per-surface proxy the runtime owns. Adding a surface kind SHALL require only a new
adapter, not a change to the dispatch.

#### Scenario: Terminal kind dispatches to the terminal adapter
- **WHEN** a terminal surface is created
- **THEN** the terminal adapter spawns the command and returns the proxy the runtime stores

#### Scenario: Agent kind dispatches to the agent adapter
- **WHEN** an agent surface is created
- **THEN** the agent adapter establishes the agent lifecycle and returns the proxy the runtime stores

#### Scenario: A new kind needs no dispatch change
- **WHEN** an adapter for a new kind is registered
- **THEN** surfaces of that kind are created through the same dispatch with no change to the runtime's selection logic

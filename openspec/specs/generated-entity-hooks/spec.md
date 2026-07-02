# Capability: generated-entity-hooks

## Purpose

Generic runtime factories over the generated Tauri bindings -- `query()`, `command()`/`runCommand()`,
`subscribe()`, and the channel helpers -- as the renderer's only entity-access surface: fully typed
from the bindings, zero per-entity hand-written wrappers, write mutations declaring
`meta.invalidates` for the global settle-invalidation and cross-window broadcast.

## Requirements

### Requirement: Entity access goes through generic runtime factories over the generated bindings

The renderer SHALL reach entity commands through generic runtime factories in
`@tillerd/client-bindings` — `query(key, args)` producing Query options, `command(key)`/
`runCommand(key, args)` producing mutations and imperative calls, `subscribe(key)` and the channel
helpers for streams — all typed from the generated bindings so a key or argument-shape mismatch is
a compile error. Write mutations SHALL declare `meta.invalidates`; the global `MutationCache`
performs the settle-invalidation and the cross-window broadcast.

#### Scenario: A new command is consumable with no hand-written per-entity code

- **WHEN** a new conforming command lands in the bindings
- **THEN** `query()`/`command()` reach it immediately with full types and no per-entity wrapper

#### Scenario: A mistyped key or argument fails the build

- **WHEN** renderer code names a command key or argument shape that the bindings do not declare
- **THEN** type-checking fails; there is no untyped fallback path

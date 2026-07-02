## REMOVED Requirements

### Requirement: The TanStack hook surface is generated from the bindings

**Reason**: The build-time hook emitter was abandoned (autogen-sdk change record, tasks 4.3-4.6);
the shipped design is generic runtime factories over the generated bindings, not emitted hooks.

### Requirement: Every command gets a generated hook of the appropriate class

**Reason**: Superseded by the runtime-factory design — commands are reached through the generic
`query()`/`command()` factories and channel helpers, not per-command generated hooks.

### Requirement: Optimistic behavior comes from a verb convention, not per-entity config

**Reason**: Optimistic behavior is declared at the call site over the factory result; the global
`MutationCache` meta-invalidation remains, but no verb->strategy emitter table exists.

### Requirement: Non-conforming commands are handled by explicit overrides

**Reason**: No emitter, no overrides map.

### Requirement: Generated hooks cannot drift

**Reason**: There is no emitted hook file to drift; drift-guarding applies to the generated
bindings (`generated-ipc-bindings`), which remain the single generated artifact.

## ADDED Requirements

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

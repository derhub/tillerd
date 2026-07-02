## ADDED Requirements

### Requirement: The TanStack hook surface is generated from the bindings

A build-time emitter SHALL produce the committed TanStack hook surface — query hooks (list/get, including infinite/paged), mutation hooks (create/rename/archive/delete/reorder/…), and event subscriptions — by reading the generated bindings and grouping commands and events by their `<verb><Entity>` naming. The emitted hooks SHALL be fully typed from the bindings. Per-entity hand-written configuration SHALL NOT be required for a conforming command.

#### Scenario: A conforming command yields a hook with no per-entity config

- **WHEN** the bindings contain a command named `<verb><Entity>` that follows the naming convention
- **THEN** the emitter produces the corresponding typed hook
- **AND** no per-entity configuration entry is written for it

#### Scenario: A new Rust command surfaces as a hook automatically

- **WHEN** a new conforming command is added in Rust and the bindings are regenerated
- **THEN** regenerating the hooks produces its hook with no hand edit

### Requirement: Every command gets a generated hook of the appropriate class

The emitter SHALL generate a hook for every command in the bindings, choosing the hook shape by command class: query commands (`*List`/`*Get`/`*Resolve`/`*Search`/`*Count`) yield query hooks; mutation commands yield `useMutation` hooks; emitted events and the notification feed yield typed subscription hooks; `ipc::Channel` surface commands yield typed channel-subscription hooks (not Query); host/infra commands yield thin typed action wrappers. No command is silently skipped — a command whose class the emitter cannot determine SHALL fail the build, not be dropped.

#### Scenario: A non-CRUD command still gets a hook

- **WHEN** the bindings contain a surface `ipc::Channel` command or a host command
- **THEN** the emitter produces a channel-subscription hook (surface) or a typed action wrapper (host)
- **AND** it is not modeled as a Query hook

#### Scenario: An unclassifiable command fails the build

- **WHEN** a command matches no known class
- **THEN** the emitter fails rather than skipping it silently

### Requirement: Optimistic behavior comes from a verb convention, not per-entity config

The optimistic strategy for a mutation SHALL be determined by a single write-once verb→strategy convention table (e.g. rename→edit, archive/delete→remove, reorder→reorder, create→invalidate), applied uniformly across all entities. The optimistic patch field for an edit SHALL be inferred from the command's argument type rather than declared per entity. Existing mutation semantics — declared `meta.invalidates`, optimistic snapshot/apply/rollback, and the global settle-invalidate handler — SHALL be preserved.

#### Scenario: A rename hook patches the inferred field optimistically

- **WHEN** a generated rename hook runs
- **THEN** the cache is patched optimistically on the field inferred from the argument type
- **AND** the pre-mutation snapshot is restored on error

#### Scenario: Strategy is shared across entities

- **WHEN** two entities each expose an `archive` command
- **THEN** both archive hooks apply the same convention-defined optimistic strategy
- **AND** neither entity declares that strategy itself

### Requirement: Non-conforming commands are handled by explicit overrides

A command that does not follow the convention (different verb shape, bespoke optimistic logic, or a non-mechanical body) SHALL be handled by an explicit entry in a small overrides map. The overrides SHALL be the only hand-written per-command input to the emitter.

#### Scenario: An override governs a non-conforming command

- **WHEN** a command requires behavior the convention does not express
- **THEN** an overrides entry supplies that behavior
- **AND** the emitter applies the override instead of the convention default

### Requirement: Generated hooks cannot drift

The emitted hook surface SHALL be committed, and a test SHALL regenerate it and fail if the result differs from the committed file.

#### Scenario: Stale hooks fail the build

- **WHEN** the bindings, convention, or overrides change but the committed hooks are not regenerated
- **THEN** the drift-guard test fails
- **AND** regenerating the hooks makes it pass

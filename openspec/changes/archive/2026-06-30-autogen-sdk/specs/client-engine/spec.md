## ADDED Requirements

### Requirement: The per-entity TanStack surface is generated, not hand-written

The renderer's per-entity TanStack surface — query hooks (list/get, including infinite/paged), mutation hooks (create, rename, archive, delete, reorder, …), and event subscriptions — SHALL be consumed from the generated hook surface rather than hand-written one operation at a time. The generated hooks SHALL preserve the engine's existing semantics: query keys, declared `meta.invalidates`, optimistic snapshot/apply/rollback, and the global settle-invalidate handler.

#### Scenario: The renderer uses generated query and mutation hooks

- **WHEN** a component needs an entity's list or a create/rename/archive/delete/reorder mutation
- **THEN** it imports the generated hook
- **AND** no hand-written per-operation hook for that entity exists in the renderer

#### Scenario: Generated hooks keep optimistic and invalidation behavior

- **WHEN** a generated rename, reorder, or archive hook runs
- **THEN** the cache updates optimistically and rolls back on error
- **AND** on success only the declared `meta.invalidates` keys are invalidated through the global handler

#### Scenario: Hook argument and result types come from the generated bindings

- **WHEN** a generated hook is called
- **THEN** its argument and result types originate from the generated bindings
- **AND** a backend type change surfaces as a type error at the call site

## ADDED Requirements

### Requirement: Every persisted entity is stored through a per-entity store over a swappable backend

The orchestrator SHALL access each persisted entity through a per-entity async store struct -- the
domain aggregates (`Workspaces`, `Projects`, `Sessions`, `Surfaces`) and the operational entities
(`Commands`, `Settings`, `Notifications`, `LaunchTemplates`) -- that holds a closed `Backend` enum
and dispatches storage to the selected variant. No multi-entity store SHALL survive the restructure
(`schema_version` is the lone exception: a meta/migration fn, not a store). The store API SHALL be
identical regardless of which `Backend` serves the entity, so a backend swap is transparent to callers.

#### Scenario: Round-trip is identical across backends

- **WHEN** an entity is created, updated, listed with a filter, and archived through the `Memory`
  backend, and the same operations run through the `Fs` backend
- **THEN** both backends produce identical observable results (same entity values, same list order,
  same archived/visible state)

#### Scenario: A typed filter is pushed to the backend

- **WHEN** `list` is called with a typed per-entity `Filter` (e.g. by parent workspace, excluding
  archived)
- **THEN** the store returns only the matching entities, resolved by the backend rather than by the
  caller

#### Scenario: Operational entities have their own stores

- **WHEN** a command, setting, notification, or launch template is read or written
- **THEN** it goes through that entity's own store (`Commands`/`Settings`/`Notifications`/
  `LaunchTemplates`), not a shared operational facade

### Requirement: The backend is selected at the composition root

The concrete `Backend` serving each entity SHALL be chosen at the composition root, not baked into
the stores or call sites. Selecting a different backend SHALL require no change to the per-entity
stores or their callers.

#### Scenario: Domain entities follow the configured backend

- **WHEN** the orchestrator is composed with the `Fs` backend for domain entities
- **THEN** domain reads and writes go to the filesystem store
- **WHEN** it is composed with the `Memory` backend instead
- **THEN** the same store API operates in memory, with no change to store or call-site code

### Requirement: The composition root owns a `Storage` aggregate; consumers depend only on the stores they use

The composition root SHALL build the per-entity stores into a single `Storage` aggregate it owns.
Leaf consumers SHALL receive only the concrete stores they call, not the whole aggregate, so each
consumer's storage dependencies are explicit and least-privilege. No `Store`/`CompositeStore` trait
object SHALL be threaded through consumers.

#### Scenario: A consumer holds only the stores it uses

- **WHEN** a leaf consumer (e.g. the settings host) is constructed
- **THEN** it receives only the concrete stores it calls (e.g. `Settings`), not the `Storage`
  aggregate or a `dyn Store` handle

### Requirement: Cross-aggregate session creation goes through a coordinator, not a store

A standalone coordinator owned by the composition root SHALL create a session from a launch template
-- resolving `launch_template` -> spec, then materializing the session through the `Sessions` store.
This spans two aggregates. The `Sessions` store SHALL NOT depend on `LaunchTemplates`, and the
coordination SHALL NOT be duplicated in host controllers.

#### Scenario: Templated session creation resolves through the coordinator

- **WHEN** a session is created with a `template_id`
- **THEN** the coordinator resolves the template to a spec and then creates the session through the
  `Sessions` store
- **AND** the `Sessions` store itself has no dependency on `LaunchTemplates`

### Requirement: Storage behavior is preserved across the restructure

The restructure SHALL NOT change observable storage behavior. Every storage scenario that held
before the restructure SHALL hold after it.

#### Scenario: The existing storage suite passes unchanged

- **WHEN** the pre-restructure storage scenario suite (workspace/project/session/surface CRUD,
  ordering, rename, archive, placement uniqueness, seeding) runs against the per-entity stores
- **THEN** every scenario passes with assertions unchanged (only adapted to the async signatures)

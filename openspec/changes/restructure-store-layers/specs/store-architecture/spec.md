## ADDED Requirements

### Requirement: Domain entities are stored through per-entity stores over a swappable backend

The orchestrator SHALL access each domain aggregate through a per-entity async store struct
(`Workspaces`, `Projects`, `Sessions`, `Surfaces`) that holds a closed `Backend` enum and dispatches
storage to the selected variant. The store API SHALL be identical regardless of which `Backend`
serves the entity, so a backend swap is transparent to callers.

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

### Requirement: The backend is selected at the composition root

The concrete `Backend` serving each entity SHALL be chosen at the composition root, not baked into
the stores or call sites. Selecting a different backend SHALL require no change to the per-entity
stores or their callers.

#### Scenario: Domain entities follow the configured backend

- **WHEN** the orchestrator is composed with the `Fs` backend for domain entities
- **THEN** domain reads and writes go to the filesystem store
- **WHEN** it is composed with the `Memory` backend instead
- **THEN** the same store API operates in memory, with no change to store or call-site code

### Requirement: Storage behavior is preserved across the restructure

The restructure SHALL NOT change observable storage behavior. Every storage scenario that held
before the restructure SHALL hold after it.

#### Scenario: The existing storage suite passes unchanged

- **WHEN** the pre-restructure storage scenario suite (workspace/project/session/surface CRUD,
  ordering, rename, archive, placement uniqueness, seeding) runs against the per-entity stores
- **THEN** every scenario passes with assertions unchanged (only adapted to the async signatures)

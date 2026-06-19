## Why

The orchestrator's data code violates separation of concerns and is not reviewable:
`persistence/tree/mod.rs` is 2559 lines (io + slug + datetime + index + four aggregates + ~750
lines of tests); `mod.rs` (576) mixes every id, entity, enum, and both trait definitions;
`memory.rs` (1067) is a parallel re-implementation of the domain rules; and the
`DomainStore`/`OperationalStore` traits are ornamental (defined, one impl, consumed as concretes).

This is **R1a of a sliced orchestrator re-architecture** into honest, reviewable layers, applying
`rust-best-practices` + `my-code-style`. The storage direction is set by **ADR-0035** (domain
stores over a swappable, **enum-dispatched** async backend; hand-editable files + a KV listing
cache for scale; supersedes ADR-0033). R1a is the **structural, behavior-preserving** step: it
lands the layers, the per-entity stores, and the `Backend` enum with the existing backends reworked
behind it. The scale work (KV cache, lazy-load, `mtime` revalidation) is **R1b**; it ships next, on
its own, so each PR stays reviewable. R1a reworks in place on PR #41.

## Layers

```
orchestrator/src/
  entities/   entity types + value-object ids + enums   (pure; no deps)
  infra/      concrete backends: fs.rs (+ slug/atomic_io/index/datetime helpers), sqlite.rs, memory.rs
  store/      per-entity async stores (domain + operational) + the closed `Backend` enum + the `Storage` aggregate
  app/        use cases  (R2)
```
No generic `Repository` trait, no trait objects, no `async-trait`. A closed `enum Backend { Fs,
Sqlite, Memory }` (`Postgres` later) wraps the concrete backends; each per-entity store struct holds
a `Backend` and dispatches by `match`. Async fns work natively on the concrete structs. Dependency
flow is acyclic: `entities <- infra`, `entities + infra <- store`, `store <- app`.

Every persisted entity gets its own store -- the four domain aggregates
(`Workspaces`/`Projects`/`Sessions`/`Surfaces`) **and** the operational entities
(`Commands`/`Settings`/`Notifications`/`LaunchTemplates`); `schema_version` is a small meta/migration
fn, not a store. The composition root builds them all into a `Storage` aggregate it owns; leaf
consumers receive only the concrete stores they call (least-privilege), not the whole bundle.

## Slice plan (separate changes/PRs, in order)

- **R1a -- data layer, structural (this change)**: `entities/`, `infra/` backends, `store/` per-entity
  stores + `Backend` enum (`fs`/`sqlite`/`memory` reworked); dissolve the dual traits; tests moved
  out. Behavior-preserving.
- **R1b -- scale**: the `fs` backend's KV listing cache + lazy-load + `mtime` revalidation (ADR-0035).
- **R2 -- `app/`**: a use-case layer; relocate `create_session` coordination + archive/soft-delete
  policy into application services.
- **R3 -- surface runtime**: daemon-pty / gate / process clients -> `infra/` adapters; spawn/attach/
  close -> `app/` use cases.
- **R4 -- launch**: launch executor -> `app/` use case; launch-spec types -> `entities/`.

Each builds on the last and is independently reviewable.

## What Changes (R1a)

- **`entities/` -- entity types, moved out of `persistence/`.** Plain structs + newtype value-object
  ids + enums (`Workspace`/`Project`/`Session`/`Surface` + `New*` drafts + ids + `SourceKind` etc.),
  pure data with zero infra deps -- the shared lowest layer.
- **`infra/` -- concrete backends.** `fs.rs` (the current tree store reworked), `sqlite.rs`,
  `memory.rs`, each a concrete async struct; sync work wraps via `spawn_blocking`. `fs.rs` plumbing
  extracts to sibling helpers (`slug.rs`, `atomic_io.rs`, `index.rs`, datetime); `sqlite.rs` /
  `memory.rs` stay flat. Backends keep today's archived-state representation (fs under `.archive/`,
  sqlite a column). **No KV cache / lazy-load / mtime in R1a -- that is R1b.**
- **`store/` -- per-entity async stores + the `Backend` enum + the `Storage` aggregate.** A closed
  `enum Backend { Fs(..), Sqlite(..), Memory(..) }` wraps the `infra` backends. One concrete async
  store struct **per entity** -- the four domain aggregates (`Workspaces`, `Projects`, `Sessions`,
  `Surfaces`) and the operational entities (`Commands`, `Settings`, `Notifications`,
  `LaunchTemplates`) -- holds a `Backend` and exposes that entity's operations
  (`get / list(&Filter) / create / update / delete` plus entity-specifics) dispatching by `match`.
  `schema_version` is a small meta/migration fn, not a store. `Filter` is a declarative, typed
  per-entity struct (`#[derive(Default)] ProjectFilter { workspace: Option<WorkspaceId>,
  include_archived: bool }`) the backend pushes down. This **dissolves `DomainStore` /
  `OperationalStore` / `CompositeStore`**; which `Backend` serves which entity is a composition-root
  choice. The root builds the stores into a plain `Storage` aggregate struct it owns; **leaf
  consumers take only the concrete stores they use** (`surface_api` <- `Surfaces` + `Sessions`,
  `settings_host` <- `Settings`, ...), never the whole bundle -- interface-segregation, not a new facade.
- **Cross-aggregate coordination forced out by dissolving the facade** (`create_session` resolving a
  `launch_template` -> spec then materializing the session; the archive decision) becomes a
  standalone **coordinator fn** the composition root owns -- **not** a method on the `Sessions` store
  (which must not depend on `LaunchTemplates`) and **not** duplicated in the tauri hosts. Both real
  callers (`workspace_host`, `surface_api`) delegate to it. R1a parks it at the root; R2 promotes it
  to an `app/` use case -- behavior-identical here.
- **Move tests out of large modules**: per responsibility into sibling `#[cfg(test)]` files for
  internal items, and `crates/orchestrator/tests/` integration tests for contract-level behavior.
- **No behavior change.** Every existing unit test + the e2e suite stays green; assertions unchanged,
  only relocated or adapted to the async signatures.

Out of scope: KV cache / lazy-load / mtime (R1b); `app/` (R2); surface (R3); launch (R4); any
observable behavior change; the remaining 0.0.15 roadmap slices.

## Capabilities

### New Capabilities

- `store-architecture`: the layered data architecture -- entities in `entities/`, concrete backends
  in `infra/`, per-entity async stores over a closed `Backend` enum in `store/`; storage swappable
  per entity at the composition root; the existing storage behavior preserved across the restructure.

### Modified Capabilities

<!-- None -- the behavior (requirements) of snapshot-tree-store is unchanged; this change is
     structural, captured as the new store-architecture capability. -->>

## Impact

- **Code (`crates/orchestrator/src/`)**: new `entities/` (types out of `persistence/mod.rs`); new
  `store/` (per-entity stores + `Backend` enum); `persistence/` removed -- backends move to `infra/`
  (`fs`/`sqlite`/`memory` + fs helpers), the dual traits + `CompositeStore` deleted; the composition
  root rewired to assign entity -> `Backend` and host the forced-out coordination. The sync `Store`
  API becomes async (sync backends wrap via `spawn_blocking`); every store call site across the
  orchestrator and tauri host gains `.await`, and sync tauri commands become async.
- **Tests**: unchanged assertions, must stay green; relocated per concern; possibly a new
  `crates/orchestrator/tests/`.
- **Standards applied**: `rust-best-practices` + `my-code-style`.
- **Scale**: R1a is the structural refactor; bounded and reviewable. Tasks phased per module.
- **Branch**: reworks in place on `feature/snapshot-tree-storage` (PR #41).

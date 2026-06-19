## 1. `entities/` -- extract types

- [ ] 1.1 Move the domain types out of `persistence/mod.rs` into `entities/` (one module per
  aggregate: `workspace`/`project`/`session`/`surface`) -- structs, newtype ids, enums, `New*`
  drafts; pure, no infra deps. Update imports; keep `cargo build` green.

## 2. `infra/` -- backends as concrete structs

- [ ] 2.1 Move the backends out of `persistence/` into `infra/`: rename the tree store -> `fs.rs`
  (`FsBackend`) and extract its plumbing to sibling helpers (`slug.rs`, `atomic_io.rs`, `index.rs`,
  `datetime.rs`); `sqlite.rs` (`SqliteBackend`); `memory.rs` (`MemoryBackend` -- one in-memory impl,
  not a parallel `Store`). Preserve current behavior exactly. Move each module's `#[cfg(test)]` into
  sibling test files (or `crates/orchestrator/tests/` for contract-level).

## 3. `store/` -- `Backend` enum + per-entity async stores

- [ ] 3.1 `store/backend.rs`: closed `enum Backend { Fs(FsBackend), Sqlite(SqliteBackend),
  Memory(MemoryBackend) }` exposing the low-level async storage primitive (dispatch by `match`); sync
  backends wrap blocking work via `spawn_blocking`.
- [ ] 3.2 One per-entity async store struct per entity -- domain (`Workspaces`/`Projects`/`Sessions`/
  `Surfaces`) and operational (`Commands`/`Settings`/`Notifications`/`LaunchTemplates`) -- holding a
  `Backend`, exposing `get / list(&Filter) / create / update / delete` + entity-specifics (reorder,
  archive-representation), with `#[derive(Default)]` typed per-entity `Filter` structs pushed to the
  backend. The stores own the invariants that were DB constraints (placement uniqueness, archive
  cascade). `schema_version` stays a small meta/migration fn on the sqlite backend, not a store.

## 4. Wire-up -- dissolve the facade

- [ ] 4.1 Delete `DomainStore`/`OperationalStore`/`Store`/`CompositeStore`, the `persistence/`
  module, and the `Arc<dyn Store>` threaded through boot/surface/launch/hosts. Rewire the composition
  root to construct backends, build every per-entity store with the chosen `Backend`
  (domain->`Fs`, operational->`Sqlite`, tests->`Memory`), and bundle them into a `Storage` aggregate
  it owns (`boot<F: FnOnce() -> Result<Storage>>`, `Orchestrator { storage: Arc<Storage> }`).
- [ ] 4.2 Add the standalone `create_session(draft, &LaunchTemplates, &Sessions)` coordinator fn at
  the root (template->spec resolution; NOT a `Sessions` method, NOT in hosts); both callers
  (`workspace_host`, `surface_api`) delegate to it. Formalized to `app/` in R2.
- [ ] 4.3 Update consumers to take only the concrete stores they use (least-privilege), not the whole
  `Storage`: `surface_api`/`launch_executor` <- `Surfaces`+`Sessions`, `settings_host` <- `Settings`,
  `notification_host` <- `Notifications`, `workspace_host` <- `Commands`+`LaunchTemplates`+coordinator.
  Adapt call sites to the async store API (`.await`).

## 5. Verify gate (fix-all)

- [ ] 5.1 Run the full verify suite to green: `fmt`, `clippy --all-targets --locked -D warnings`,
  unit tests (relocated/adapted, all green -- incl. the `store-architecture` scenarios: cross-backend
  round-trip parity, composition-root selection, behavior preserved), and the affected e2e
  (boot/create/resume). No module left ~1000 lines; assertions unchanged beyond `.await`. Fix any
  gap and re-run until clean.

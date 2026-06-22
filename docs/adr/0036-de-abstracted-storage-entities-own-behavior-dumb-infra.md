# 0036. De-abstracted storage: sqlite via sqlx, per-entity repos, CQS app layer

- Status: accepted, supersedes ADR-0035
- Supersedes: ADR-0035
- Date: 2026-06-21

## Context

ADR-0035 layered the data layer into `entities/ + infra/ + store/ + app/` over a closed
`Backend { Fs | Sqlite | Memory }` enum, with domain entities persisted as a human-readable slug-tree.
In practice the structure fused persistence with domain logic and added indirection that earns nothing:

- `infra/fs` has a file per entity and bakes domain rules into persistence: `rename_workspace` derives
  slugs, decides re-slugging, moves directories, reindexes; `delete_workspace` hardcodes the
  Default-guard and reassign cascade.
- The eight `store/<entity>` wrappers are one-line forwarders; the `Backend` enum is 629 lines with 22
  `wrong_backend` arms; `MemoryBackend` (1119 lines) is a test-only double duplicating those rules.
- The slug-tree turns a rename into a directory move + collision scan + subtree reindex -- heavy
  machinery for data that is inherently relational.

ADR-0023 (workspace data model) is honored at the model level; its slug-tree representation is revised.

## Decision

Four layers, each with one job; domain data in sqlite via `sqlx`; operations as CQS objects on a bus.

- **`entities/` are pure domain.** Types plus rules -- the `is_default`/`is_unfiled` guards, the
  rename-sets-`title_source`-to-`Custom` rule, the cascade policy -- with no infra trait and no I/O.
- **`infra/` is all infrastructure: per-entity async `sqlx` repositories plus the surface runtime.**
  One repo per entity, with typed async `create`/`get`/`list(parent, page)`/`update`/`delete` that take a
  sqlx executor (a pool or transaction ref, so several repos share one transaction), owning its table,
  columns, and `Row -> Entity` mapping. Nesting is a `parent_id` column; rename/move/archive are
  `UPDATE`s; cascades are `UPDATE`/`DELETE`. `sqlx` 0.9 (async, compile-time-checked queries) is the
  driver -- **not** an ORM (SeaORM/Diesel rejected: entity-mapping plus multi-database swappability this
  change removes). The earlier goal of a single entity-agnostic storage trait is dropped: typed columns,
  domain ordering, and pagination need entity shape, so per-entity repos are the honest fit. The surface
  runtime (PTY proxies, daemon client; from `surface/runtime.rs`) also moves here, behind a
  `SurfaceRuntime` port, as an infrastructure concern. The `Backend` enum, the `store/` module, the
  `infra/memory` backend, and the slug-tree machinery are deleted.
- **`shared/` holds reusable building blocks**, not a storage abstraction: `fs` (file utils, used by
  user-config), `kv` (a `Kv` trait with async `put`/`get`+TTL and `SqliteKv` + `MemoryKv` impls), `page`
  (`Page` + `Listing<T>`), `datetime`, `errors`, and the CQS machinery (`Command<Cx>`/`Query<Cx>` traits
  and `Bus<Cx>`). No generic `Repository` trait -- sqlite is entity-aware, so it would have no honest
  implementor.
- **`app/` is a CQS layer of command/query objects.** Each operation is a type implementing
  `Command<Ctx>` (mutate, returns `()`) or `Query<Ctx>` (read, returns `Out`), async `handle(&self, &Ctx)`,
  dispatched through a thin `Bus<Ctx>` (`execute<C>`/`query<Q>`, static generics; the bus carries
  telemetry but **no transaction**). Operation names use the product's ubiquitous language, not generic
  CRUD: `New*` (absorbing the entity draft structs), `Rename*`/`Discard*`/`MoveProject`/`Archive*`/
  `SpawnSurface`; reads are descriptive `Get`/`List` (`GetWorkspaceById`). **The transaction boundary is
  per command, not the bus**: a command opens one only when it spans multiple writes, via a
  `Ctx::transaction(|tx| …)` helper (commit on `Ok`, explicit awaited rollback on `Err`); single writes
  and runtime-only ops use no transaction. A handler reads load -> entity rule -> persist (-> cascade);
  cross-entity cascades live in the parent's command. `Ctx` holds the pool, kv, config root, and the
  `SurfaceRuntime` port, exposing `db()`/`runtime()`/`transaction()`; repos are executor-passing (take a
  pool or tx ref). `boot/` opens the pool, builds `Ctx` + `Bus`, and injects it.
- **Transport is thin adapters over a transport-agnostic core.** The `Command`/`Query` types and `bus`
  know no transport; the core structs carry no transport derive. Each transport owns its shim macro in
  its own layer: the tauri crate's declarative `transport_command!`/`transport_query!` (`type => action`)
  generates the per-operation `#[tauri::command]` shim (wire unchanged: `invoke('rename_workspace',
  { .. })`) and registers it via `inventory`, so Tauri keeps native routing, arg typing, and per-command
  ACL. A single `invoke('command', { action, payload })` gateway was rejected -- it relocates per-command
  authorization out of Tauri's ACL into hand-built app code (more maintenance, wider security surface).
  The future web server owns its own macro (axum routes) over the same commands and bus. Wiring splits by
  ownership: orchestrator `boot::build_bus` builds the core; each transport owns its server setup.
- **User-config stays file-based** through `shared::fs`. Tauri command signatures, the dynamic ACL, and
  the wire protocol are unchanged; hosts build a command/query and call the bus.

## Consequences

- Each concern has one home: rules on entities; typed persistence in per-entity repos; reusable
  primitives in `shared/`; operations as CQS objects on a bus. The dead enum arms, the wrapper layer, the
  duplicate backend, and the slug-tree machinery are gone.
- Relational domain data is queried relationally (parent-id filters, pagination, ordering) instead of
  walked as a directory tree, removing the rename-moves-directories complexity.
- A new dependency (`sqlx`) and a sqlite schema/migration for the domain tables; `sqlx`'s compile-time
  query checking catches schema drift at build time. `rusqlite` is dropped.
- The domain on-disk format breaks (slug-tree -> sqlite) with no migration. Accepted: pre-v1, no released
  users. Human-readable/git-navigable domain directories are lost; user-config remains file-based.
- The IPC contract, the dynamic ACL, and the wire protocol are unchanged.
- Out of scope and deferred: Stronghold secrets, the settings-profile cascade, the state-model contract
  (ADR-0034), and a boxed-command queue.

# Design: storage-de-abstraction

## Context

ADR-0035 layered the data layer into `entities/ + infra/ + store/ + app/` over a `Backend` enum, with
domain entities on a slug-tree. It fused persistence with domain logic (`infra/fs` derives slugs, moves
directories, hardcodes guards/cascades), added one-line `store/` wrappers and a 629-line dispatch enum,
and kept a 1119-line `MemoryBackend` test double duplicating the domain rules. The slug-tree turns a
rename into a directory move + collision scan + subtree reindex.

ADR-0023 (workspace data model) is honored at the model level; its slug-tree representation is revised. A
new ADR supersedes ADR-0035.

## Goals

- Four clear layers, each with one job: `entities/` (rules), `shared/` (reusable building blocks),
  `infra/` (per-entity persistence), `app/` (operations as CQS objects), wired in `boot/`.
- Relational domain data in sqlite via `sqlx`; user-config in files.
- Operations read top-to-bottom and compose by adding/removing a line.

## Non-Goals

- The broader 0.0.15 feature set (Stronghold secrets, state-model contract). *(Settings + profile
  management, including the cascade, are in scope -- see the Setting/Profile operations.)*
- Migrating existing on-disk slug-tree data (pre-v1; the format break is taken).
- A command queue / `Box<dyn Command>` dispatch path (additive later).
- An ORM, or a generic entity-agnostic storage abstraction.

## Decisions

### D1 -- `entities/` are pure domain

Entities hold types and rules only -- the `is_default`/`is_unfiled` guards, the
rename-sets-`title_source`-to-`Custom` rule, the archive/delete cascade policy, the
**archive-requires-idle** invariant, and the **prebuilt-is-immutable** guard (a `Prebuilt` command or
template rejects rename/edit/discard, like Default/Unfiled) -- with no infra-facing trait and no I/O. A rename is an entity
method that mutates the entity; persistence is the caller's job.

**Archive requires idle.** A session is *idle* when it has no running (`live`) surfaces. Archiving is
rejected unless every session in scope is idle: `ArchiveSession` requires that session idle,
`ArchiveProject` requires all its sessions idle, `ArchiveWorkspace` requires every session under it idle.
The idle predicate is a pure entity rule over surface state; the command supplies the current surface
statuses (read from the surface repo / runtime) and the entity decides -- the entity itself does no I/O.

### D2 -- `infra/` is per-entity, entity-aware async `sqlx` repositories

Each domain entity gets a repository (`WorkspaceRepo`, `ProjectRepo`, ...) exposing typed async
`create`/`get`/`list(parent, page)`/`update`/`delete` and owning its table, columns, and
`Row -> Entity` mapping. Persistence is `sqlx` 0.9 (async, compile-time-checked queries; **not** an ORM).
Nesting is a `parent_id` column resolved by `list`; rename/move/archive are `UPDATE`s.

Repo methods **take a sqlx executor** (`impl SqliteExecutor`), they do not own a `SqlitePool`. Because
`Executor` is implemented by both `&SqlitePool` and `&mut Transaction`, the same method serves a
single-statement call (pass the pool) and a multi-repo atomic command (pass a shared transaction) -- this
is what makes cross-repo cascades atomic (see D4). So a repo is a unit struct (or module) of
executor-passing functions, not a pool-bound handle.

The earlier ambition of a single entity-agnostic storage trait is dropped deliberately: typed columns,
domain ordering (e.g. `sort_order`), and pagination need entity shape, and forcing them through an
opaque-bytes interface either leaks structure or pushes filtering/ordering into callers. Entity-aware
per-entity repos are the honest fit. The generic building blocks that *can* stay agnostic live in
`shared/` (below).

### D3 -- `shared/` holds reusable building blocks, not a storage abstraction

`shared/` carries anything used more than once:
- `fs` -- file read/write/list/delete utils (used by user-config); a utility module, not a store.
- `kv` -- a schemaless `Kv` trait (`put(key, &[u8], opts{ttl})`, `get(key) -> Option<Vec<u8>>`, async)
  with two impls, `SqliteKv` (sqlx-backed) and `MemoryKv` (in-memory). Two impls justify the trait.
- `pagination` -- `Page { All, Offset, Cursor }` and `Listing<T> { items, next }` for repo pagination.
- `cqs` -- the generic CQS contracts (D4).
- `bus` -- the generic dispatcher (D4).
- `datetime`, `errors`.

There is no generic `Repository` trait; sqlite is entity-aware (D2), so a shared opaque-store contract
would have no honest implementor.

### D4 -- `app/` is CQS command/query objects dispatched through a `Bus`

Following Meyer's CQS, every operation is a value implementing one of two generic contracts in `shared/`.
Both get only the context; **the transaction (if any) is the command's concern, not the bus's**:

```rust
pub trait Command<Cx>: Send + 'static { async fn handle(&self, cx: &Cx) -> Result<()>; }
pub trait Query<Cx>:   Send + 'static { type Out: Send; async fn handle(&self, cx: &Cx) -> Result<Self::Out>; }
```

**The transaction boundary is per command, not on the bus.** Not every command needs one: a
single-statement mutation is already atomic in sqlite, a runtime-only op (e.g. `input`/`resize`) touches
no database, and `SpawnSurface` mixes a row write with a non-transactional PTY spawn. A blanket
tx-per-command on the bus would wrap those in pointless (or wrong) transactions. So a command opens a
transaction **only when it needs to span multiple writes**, via a `Ctx::transaction(|tx| …)` helper that
commits on `Ok` and **explicitly, awaited-rolls-back on `Err`** (sqlx rolls back on `Drop`, but `Drop`
can't `.await` and reports no failure -- the helper gives deterministic timing and a loggable result).
The helper centralizes that discipline so commands don't repeat it.

```rust
impl Ctx {
    pub fn db(&self) -> &SqlitePool { &self.db }
    // opt-in unit of work: commit on Ok, awaited rollback on Err
    pub async fn transaction<T>(&self, f: impl AsyncFnOnce(&mut SqliteTx<'_>) -> Result<T>) -> Result<T> { … }
}
```

The bus stays a **thin dispatcher** -- it no longer owns transactions; it keeps only the cross-cutting
telemetry (the span + error event):

```rust
impl<Cx> Bus<Cx> {
    pub async fn execute<C: Command<Cx>>(&self, c: C) -> Result<()> {
        c.handle(&self.cx).await.inspect_err(|e| self.record(e))   // telemetry only; no tx
    }
    pub async fn query<Q: Query<Cx>>(&self, q: Q) -> Result<Q::Out> { q.handle(&self.cx).await }
}
```

So: a single-write or runtime command uses `cx.db()` / the runtime port directly (no tx); a multi-repo
cascade wraps its writes in `cx.transaction(|tx| …)`. Atomicity is declared where it's actually needed.

Dispatch is **static** generics, per rust-best-practices ch.6 ("static where you can; avoid boxing too
early"): operations are called with their concrete type in hand, so there is no `Box<dyn Command>` /
`execute_boxed` and no heterogeneous storage. A boxed-command queue is additive later if needed; queries
can never be boxed uniformly (the associated `Out` type has no single return). Handlers can use native
`async fn` in traits unless the runtime needs `Send` handler futures (then `async-trait` or
`trait_variant` -- a Send-ergonomics call settled at wiring time, not a design choice).

`app/` holds one op type per entity file, named in the product's ubiquitous language -- **not generic
CRUD**: create is `New*` (`NewWorkspace`, absorbing the old draft struct), then `RenameWorkspace`,
`DiscardWorkspace`, `MoveProject`, `ArchiveSession`, `SpawnSurface`, `ApplyLaunchSpec`, ...; reads are
descriptive `Get`/`List` queries (`GetWorkspaceById`, `ListWorkspaces`, `ListProjectsByWorkspace`).
(`settings.rs` for config over `shared::fs`.) A
command reads top-to-bottom: load
-> entity rule -> persist through the injected `tx` (-> cascade). Cross-entity cascades live in the
parent's command (`DiscardWorkspace` reassigns projects then deletes the workspace, all on the one `tx`).
Two rules follow from bus-owned transactions: queries take no `tx` (reads never hold a write lock), and a
command never re-dispatches through the bus (that would nest transactions) -- it composes repos and
entity logic directly.

### D5 -- `Ctx` is lazy; `boot/` wires everything

`Ctx` holds only real resources -- the `SqlitePool`, the `SqliteKv`, the config root, and the runtime
port (`Arc<dyn SurfaceRuntime>`). It exposes the pool (`cx.db()`) for queries/single-statement commands,
the runtime port (`cx.runtime()`), and an opt-in `cx.transaction(|tx| …)` helper (commit on `Ok`,
explicit awaited rollback on `Err`) for commands that span multiple writes. Repos are unit structs whose
methods take whatever executor they are handed (`cx.db()` or `&mut *tx`), so `Ctx` keeps no pre-built
repo aggregate and nothing is bound to a single connection. `Ctx` is cheap to clone and `Send + Sync`, so it survives
`.await` and Tauri's `manage`. `boot/` (the composition root) opens the pool, constructs `Ctx` and the
`Bus<Ctx>`, and injects the bus into hosts and internals.

Dependency injection is plain constructor injection from this single composition root -- no globals, no
service locator, no DI framework. Storage and config stay **concrete** because their real
implementations are cheap to stand up in tests (`:memory:` `SqlitePool`, a tempdir for `fs`), so they
need no trait purely for mocking. Traits-and-`dyn` are reserved for genuine side-effecting **ports**
(process launcher, session activator, clock) that cannot be cheaply instantiated -- those are injected as
`Arc<dyn Port>` with a test double.

### D6 -- User-config is file-based; callers migrate; IPC frozen

Settings/config/theme/keybindings/profile are read/written through `shared::fs`. Tauri
`#[tauri::command]` signatures, the dynamic ACL, and the wire protocol are unchanged; the `do_*` host
functions and the orchestrator internals build a command/query and call `bus.execute`/`bus.query`.

### D7 -- Transport = per-command typed shims; the core is transport-agnostic

The CQS objects and `bus` carry no transport knowledge; each transport is a thin adapter. For tauri the
adapter is **one typed `#[tauri::command]` shim per operation** -- the wire stays
`invoke('rename_workspace', { id, name })` (the action is the tauri command name). Tauri has no
catch-all command, so this clean wire *requires* per-command handlers; the upside is that Tauri keeps
doing routing, argument typing, and per-command ACL/capabilities. A single `invoke('command',
{ action, payload })` gateway was rejected: it would force a hand-built dispatcher and relocate
per-command authorization out of Tauri's ACL into app code -- more to maintain and a wider security
surface -- against rust-best-practices ch.6 ("lean on the framework; static where you can").

Each transport **owns its shim macro, in its own layer.** The core command/query structs in `app/` stay
pure -- `#[derive(Serialize, Deserialize)]` (the ID newtypes too) + `impl Command`/`Query`, no transport
derive -- so the core never depends on a transport (a transport derive on the shared struct is impossible
without that coupling). The tauri crate owns three declarative `macro_rules!` macros that **list** the ops
by `(type => action)` and generate the `#[tauri::command]` shim:
- `transport_command!(Cmd => "action")` -- deserialize the wire payload into `Cmd`, `bus.execute`, return
  `()`; map `shared::Error` to the wire string.
- `transport_query!(Q => "action" [=> WireDto::from])` -- `bus.query`, map the output through an optional
  response mapper to the curated wire DTO.
- `transport_create!(Cmd => "action", returns: GetByIdQuery => WireDto::from)` -- the pure-CQS create
  pattern: mint the id transport-side, `bus.execute(Cmd { id, ..payload })`, `bus.query(GetByIdQuery{id})`,
  map to the wire DTO. (CQS stays pure -- commands return `()`; minting lives in this ONE macro, not in
  per-command hand-written shims.)

Registration is a single declarative `collect_transport!()` listing that expands to the
`generate_handler![...]` array (NOT `inventory` -- Tauri's `generate_handler!` needs the handler idents at
compile time, which runtime linker-section collection cannot provide). Host/shell commands (window/file/
log/daemon-bridge/menu/supervisor/gate) are NOT domain ops and stay hand-written, listed alongside.

**Orchestration belongs in `app/`, not the transport.** A shim is mechanical (one macro line); any
multi-step domain logic lives in the command. E.g. `DiscardProject` archives-then-discards internally and
returns `()`; the transport never chains two bus calls. The only transport-resident step is create-id
minting, forced by pure CQS (above). The curated `*Response` wire DTOs (field subsets, camelCase) are the
frozen wire contract and are mapped in the macro -- a future axum transport maps the same core types to
its own wire via its **own** macro. Each framework routes natively; no shared custom dispatcher; core reused unchanged.

This also splits wiring by ownership: orchestrator `boot` builds the transport-agnostic core
(`build_bus(cfg) -> Bus<Ctx>`); the tauri app entry owns the `tauri::Builder` (`manage(bus)`,
`invoke_handler(collect_transport!())`, `run`); a server entry would own its axum `Router`. The
transport macro lives with its transport (a declarative `macro_rules!` in the tauri crate). The only
proc-macro crate is the small core-owned `tillerd-custom-macro` hosting `ErrorCode` (reads `#[error_code]`
-> `code()`). Errors carry no `level`/`category` -- they all log at `ERROR`.

### D9 -- Side effects run outside the transaction: persist intent -> effect -> record -> reconcile

A side effect (spawning a PTY, launching a process, a network call) is **never run inside a DB
transaction**. Holding sqlite's single write lock across a slow external effect would serialize all
writes behind it; and a transaction can't roll back a spawned process anyway. So a side-effecting command
follows one shape:

1. **Persist intent** -- write the desired state in a short, lock-released write (e.g. a surface row at
   status `pending`); commit immediately.
2. **Run the effect lock-free** -- call the runtime port (`cx.runtime().spawn(...)`) with no transaction
   held.
3. **Record the outcome** -- a second short write sets status `live`/`failed`.
4. **Reconcile** -- `ReconcileSurfaces` (the boot reconciler, replacing the old eager attach-all
   `resume_all`) converges actual to desired on boot and after failures, using the daemon `List` frame:
   desired-but-not-running -> respawn or mark; running-but-no-row -> kill. It does not attach proxies --
   streaming is brought up lazily per surface by `AttachSurface` when a renderer registers its Channel.

The DB is the source of truth for *intent*; the runtime is driven to match. This is chosen over
**spawn-inside-the-transaction** (holds the write lock for the spawn's duration) and over
**commit-then-compensate** (a crash between the effect and its compensating write strands state) --
reconciliation is the only crash-safe option (`ReconcileSurfaces` at boot via the daemon `List` frame).

This generalizes: any future side-effecting operation keeps the effect out of `Ctx::transaction`, persists
desired state, records the outcome, and relies on the reconciler. The cost is a lifecycle **status** on
side-effect-backed entities and a reconciler per such resource.

## Target structure

```
crates/orchestrator/src/
  entities/                 KEEP — pure domain types + rules, no I/O
    workspace.rs project.rs session.rs surface.rs
    command.rs launch_template.rs notification.rs setting.rs
    launch_spec.rs          LaunchSpec/LaunchItem/CommandRef (moved from launch/spec.rs — domain types)
  shared/                   NEW — reusable building blocks (no entity knowledge)
    fs.rs                   file read/write/list/delete utils (used by user-config)
    kv.rs                   trait Kv + SqliteKv (sqlx) + MemoryKv
    cqs.rs                  trait Command<Cx> { handle(&Cx) } / trait Query<Cx> { handle(&Cx)->Out }
    bus.rs                  Bus<Cx> { execute<C>, query<Q> } — thin dispatch + telemetry, NO tx
    pagination.rs           Page { All, Offset, Cursor } + Listing<T> { items, next }
    errors.rs               the error registry (one enum; #[derive(ErrorCode)] #[error_code("…")] → code();
                            all errors log at ERROR — no level/category)
    datetime.rs
  infra/                    ALL infrastructure: persistence + runtime I/O
    workspace.rs project.rs session.rs surface.rs        per-entity async sqlx repos
    command.rs launch_template.rs notification.rs
    migrations/             sqlx schema for the domain tables
    runtime/                surface runtime adapter behind a SurfaceRuntime port — PTY proxies +
                            daemon socket transport (moved from surface/runtime.rs + surface/transport.rs);
                            exposes the daemon `List` frame (for ReconcileSurfaces) and `Stop` (StopSurface,
                            keep record) vs `Kill` (CloseSurface, delete record) — both already in the daemon
                            protocol but not yet in the client; pushes output to a SurfaceEventSink port
                            (tauri impl = per-surface ipc::Channel)
  context.rs                Ctx { db, kv, fs_root, runtime } — exposes db() / runtime() / transaction(|tx|…)
                            (top-level: app references it, boot builds it — avoids an app↔boot cycle)
  app/                      CQS command/query objects — PURE, transport-agnostic (no transport derive)
    workspace.rs project.rs session.rs surface.rs settings.rs
                            surface.rs: SpawnSurface/CloseSurface (persist via repo + drive SurfaceRuntime port)
  boot.rs                   build_bus(cfg) -> Bus<Ctx>  (builds the core; NO tauri wiring here)
  ── surface/ and launch/ dirs are REMOVED, contents redistributed:
       surface/api.rs → app surface commands · surface/runtime.rs + surface/transport.rs → infra/runtime
       launch/executor.rs → app command (over the port) · launch/spec.rs → entities/launch_spec.rs
  ── DELETED: store/ (whole dir), infra Backend enum, infra/memory backend,
       infra/fs/ (whole dir — the slug-tree machinery: slug/index/cache/atomic_io + per-entity fs stores),
       infra/sqlite.rs (orphan old SqliteBackend; schema.rs already gone), surface/ dir, launch/ dir
       (KEEP infra/migrate.rs — the new sqlx Migrator/pool — and infra/migrations/)

crates/custom-macro/  small core-owned proc-macro crate (package `tillerd-custom-macro`)
  ErrorCode                  → reads #[error_code("…")] per variant, generates Error::code() (exhaustive)

apps/desktop/src-tauri/      OWNS the tauri transport: the macro + the Builder wiring
  (transport macro is a declarative macro_rules! here — not a proc-macro crate)
  transport macro            transport_command!/transport_query! (type => action) → #[tauri::command] shim
                             + inventory registration  (tauri-owned, lists the core command/query types)
  main/lib                   build_bus() then tauri::Builder.manage(bus).invoke_handler(collect_transport!()).run
  command_contract.rs        contract test builds the bus over a :memory: Ctx
  ── DOMAIN host modules COLLAPSE into transport_command!/transport_query! listing lines (the macro
       generates each #[tauri::command] shim from the core type): the hand-written do_* shim fns in
       workspace_host/surface_host(domain spawn/close)/settings_host/notification_host/store.rs(pref/registry)
       are REMOVED. orchestrator_host's Storage::open(fs,sqlite) → build_bus(cfg) + manage(Bus<Ctx>).
  ── KEEP as hand-written #[tauri::command] (host/shell, out of CQS scope — see "Out of CQS scope" below):
       window_host (window_close), files (file_read/file_size), diag (log_forward/list_log_files),
       bridge (daemon_connect/send/disconnect), menu, supervisor, gate_admin, daemon_session.
       These touch no domain store and are NOT macro-listed.
(future) server crate        owns ITS OWN transport macro → axum routes over the same core commands/bus
```

## Worked example -- rename and read a workspace, top to bottom

```rust
// entities/workspace.rs — pure domain; the rule lives here
pub struct Workspace { pub id: WorkspaceId, pub name: String, pub sort_order: u32 }
impl Workspace {
    pub fn rename(&mut self, name: &str) { self.name = name.trim().to_owned(); }
}
```
```rust
// shared/cqs.rs — both get only the context; the transaction (if any) is the command's concern
pub trait Command<Cx> { async fn handle(&self, cx: &Cx) -> Result<()>; }
pub trait Query<Cx>   { type Out; async fn handle(&self, cx: &Cx) -> Result<Self::Out>; }

// shared/bus.rs — thin dispatcher: telemetry only, NO transaction
pub struct Bus<Cx> { cx: Cx }
impl<Cx> Bus<Cx> {
    pub async fn execute<C: Command<Cx>>(&self, c: C) -> Result<()> {
        c.handle(&self.cx).await.inspect_err(|e| self.record(e))   // span + error event; no tx
    }
    pub async fn query<Q: Query<Cx>>(&self, q: Q) -> Result<Q::Out> { q.handle(&self.cx).await }
}
```
```rust
// infra/workspace.rs — per-entity sqlx repo; methods take an executor (pool OR tx)
pub struct WorkspaceRepo;
impl WorkspaceRepo {
    pub async fn get<'e>(exec: impl SqliteExecutor<'e>, id: &WorkspaceId) -> Result<Option<Workspace>> {
        let row = sqlx::query_as!(WorkspaceRow,
            "SELECT id, name, sort_order FROM workspaces WHERE id = ?", id)
            .fetch_optional(exec).await?;
        Ok(row.map(Into::into))
    }
    pub async fn update<'e>(exec: impl SqliteExecutor<'e>, ws: &Workspace) -> Result<()> {
        sqlx::query!("UPDATE workspaces SET name = ?, sort_order = ? WHERE id = ?",
            ws.name, ws.sort_order, ws.id).execute(exec).await?;
        Ok(())
    }
}
```
```rust
// context.rs — holds resources; exposes the pool + an opt-in transaction helper
pub struct Ctx { db: SqlitePool, kv: SqliteKv, fs_root: PathBuf, runtime: Arc<dyn SurfaceRuntime> }
impl Ctx {
    pub fn db(&self) -> &SqlitePool { &self.db }
    pub fn runtime(&self) -> &dyn SurfaceRuntime { &*self.runtime }
    // opt-in unit of work: commit on Ok, explicit awaited rollback on Err
    pub async fn transaction<T>(&self, f: impl AsyncFnOnce(&mut SqliteTx<'_>) -> Result<T>) -> Result<T> {
        let mut tx = self.db.begin().await?;
        match f(&mut tx).await {
            Ok(v)  => { tx.commit().await?; Ok(v) }
            Err(e) => { let _ = tx.rollback().await; /* log if rollback fails */ Err(e) }
        }
    }
}
```
```rust
// app/workspace.rs (orchestrator CORE) — PURE, transport-agnostic. No transport derive.

// single write → no transaction needed (one statement is atomic)
#[derive(Serialize, Deserialize)]
pub struct RenameWorkspace { pub id: WorkspaceId, pub name: String }
impl Command<Ctx> for RenameWorkspace {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut ws = WorkspaceRepo::get(cx.db(), &self.id).await?
            .ok_or(Error::WorkspaceNotFound(self.id.clone()))?;
        ws.rename(&self.name);                              // entity rule
        WorkspaceRepo::update(cx.db(), &ws).await           // one write, no tx
    }
}

// multi-repo cascade → opt into a transaction; the helper commits / awaited-rolls-back
#[derive(Serialize, Deserialize)]
pub struct DiscardWorkspace { pub id: WorkspaceId }
impl Command<Ctx> for DiscardWorkspace {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let ws = WorkspaceRepo::get(cx.db(), &self.id).await?
            .ok_or(Error::WorkspaceNotFound(self.id.clone()))?;
        ws.guard_not_default()?;                            // entity rule
        cx.transaction(|tx| async move {
            ProjectRepo::reassign(&mut **tx, &self.id, &WorkspaceId::DEFAULT).await?;
            WorkspaceRepo::delete(&mut **tx, &self.id).await
        }).await                                            // both commit or both roll back
    }
}

// side-effecting op (D9): persist intent → effect lock-free → record outcome → reconcile. NO tx around the spawn.
#[derive(Serialize, Deserialize)]
pub struct SpawnSurface { pub session: SessionId, /* kind, command, … */ }
impl Command<Ctx> for SpawnSurface {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let surface = /* build entity */;
        SurfaceRepo::create(cx.db(), &surface.pending()).await?;     // 1) intent; lock released on commit
        match cx.runtime().spawn(&surface).await {                   // 2) effect — NO lock held
            Ok(())  => SurfaceRepo::set_status(cx.db(), &surface.id, Live).await,     // 3) record
            Err(e)  => { SurfaceRepo::set_status(cx.db(), &surface.id, Failed).await?; Err(e) }
        }
        // 4) resume_all reconciles desired↔running on boot / after failure
    }
}

#[derive(Serialize, Deserialize)]
pub struct GetWorkspaceById { pub id: WorkspaceId }
impl Query<Ctx> for GetWorkspaceById {
    type Out = Option<Workspace>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> { WorkspaceRepo::get(cx.db(), &self.id).await }
}
```
```rust
// orchestrator/boot.rs — builds the transport-agnostic core; NO tauri here
pub async fn build_bus(cfg: &Config) -> Result<Bus<Ctx>> {
    let pool = SqlitePool::connect(&cfg.db_url).await?;
    let runtime = Arc::new(DaemonRuntime::connect(&cfg.socket).await?);   // infra runtime adapter
    Ok(Bus::new(Ctx { db: pool.clone(), kv: SqliteKv::new(pool), fs_root: cfg.fs_root.clone(), runtime }))
}
```
```rust
// apps/desktop/src-tauri — OWNS the tauri transport: the listing macro + the Builder wiring
transport_command!(RenameWorkspace => "rename_workspace");   // tauri-owned macro; refs the core type;
transport_command!(DiscardWorkspace => "discard_workspace");   //   generates #[tauri::command] + registers
transport_query!(GetWorkspaceById  => "get_workspace_by_id");

#[tokio::main]
async fn main() -> Result<()> {
    let bus = orchestrator::build_bus(&cfg).await?;
    tauri::Builder::default()
        .manage(bus)
        .invoke_handler(collect_transport!())   // inventory-collected shims
        .run(tauri::generate_context!())?;
    Ok(())
}
```
```ts
// frontend — typed client, namespaced per entity; the wire is still each command's action name
export const api = {
  workspace: {
    new:     (p: { name: string })            => invoke<Workspace>('new_workspace', p),
    rename:  (p: { id: string; name: string }) => invoke('rename_workspace', p),
    discard: (p: { id: string })              => invoke('discard_workspace', p),
    getById: (p: { id: string })              => invoke<Workspace | null>('get_workspace_by_id', p),
    list:    ()                               => invoke<Workspace[]>('list_workspaces'),
  },
  // project: { … }, session: { … }, surface: { spawn, close, … }
};
```

Flow: `api.workspace.rename` → `invoke('rename_workspace', …)` → tauri shim (generated by the tauri-owned
`transport_command!`) → `bus.execute` → `RenameWorkspace::handle` → `repo.get` → `Workspace::rename` →
`repo.update`. The core command stays pure; each transport lists it through its own macro. Adding an
operation = one core `struct` + `impl Command`/`Query`, then one `transport_command!` line per transport
that exposes it.

## Operations (the full command/query inventory)

Discovered from the current store methods + tauri commands + surface/launch ops, renamed to ubiquitous
language and **grouped by domain**. Legend: `C` = bus command (CQS object), `Q` = query, `io` = `app`
function that skips the bus (the Surface I/O channel). `*(new)*` marks an op with no current
implementation, added for completeness.

**Pinning** *(new)*: Workspace, Project, Session, Command, and Template each gain a `pinned` flag with
`Pin*`/`Unpin*` commands; their `List*` queries return **pinned-first**.

**Workspace**
- C `NewWorkspace` -- create a workspace
- C `RenameWorkspace` -- rename a workspace
- C `ReorderWorkspace` -- set a workspace's sort order
- C `StopWorkspaceSurfaces` -- stop every surface under the workspace -> workspace idle
- C `ArchiveWorkspace` -- archive a workspace (cascades; Default rejected; rejected unless every session under it is idle) *(new -- no workspace archive today)*
- C `RestoreWorkspace` -- restore an archived workspace *(new)*
- C `PinWorkspace` / `UnpinWorkspace` -- toggle the `pinned` flag *(new)*
- C `DiscardWorkspace` -- delete a workspace, reassigning its projects to Default (Default rejected)
- Q `ListWorkspaces` -- all workspaces, ordered
- Q `GetWorkspaceById` -- one workspace

**Project**
- C `NewProject` -- create a project in a workspace
- C `RenameProject` -- rename a project
- C `ReorderProject` -- set a project's sort order
- C `MoveProject` -- reassign a project to another workspace
- C `StopProjectSurfaces` -- stop every surface across the project's sessions -> project idle
- C `ArchiveProject` -- archive a project (cascades to its sessions and surfaces; rejected unless every session in it is idle)
- C `DiscardProject` -- hard-delete an archived project (Unfiled rejected; must be archived first)
- C `RestoreProject` -- restore an archived project *(new -- archive has no inverse today)*
- C `DuplicateProject` -- clone a project (its sessions + specs) *(new)*
- C `PinProject` / `UnpinProject` -- toggle the `pinned` flag *(new)*
- Q `GetProjectById` -- one project
- Q `ListProjectsByWorkspace` -- projects in a workspace (live + optionally archived)
- Q `SearchProjects` -- fuzzy find projects by name (sqlite-side) *(new)*

**Session**
- C `NewSession` -- create a session in a project (snapshot a template's launch spec, or empty)
- C `LaunchSession` -- instantiate a session's launch spec onto the runtime: spawn each spec item's surface, each following the D9 side-effect shape *(from `launch/executor.rs`; a session with no spec launches nothing)*
- C `RenameSession` -- rename a session (sets `title_source = Custom`)
- C `ReorderSession` -- set a session's sort order
- C `StopSessionSurfaces` -- stop all surfaces in the session (StopSurface each) -> session idle
- C `ArchiveSession` -- archive a session (cascades to its surfaces; rejected unless the session is idle -- no running surfaces)
- C `DiscardSession` -- hard-delete an archived session (must be archived first)
- C `RestoreSession` -- restore an archived session *(new)*
- C `MoveSession` -- move a session to another project (reassign `project_id`) *(new)*
- C `DuplicateSession` -- clone a session with its launch spec *(new)*
- C `PinSession` / `UnpinSession` -- toggle the `pinned` flag *(new)*
- C `ApplyLaunchSpec` -- replace a session's launch spec (the *recipe*: which surfaces exist + each one's `placement` slot)
- C `ArrangePanels` -- set a session's panel-tree geometry (the *layout*: how placements are split into panes/tabs) *(new -- a plane distinct from the launch spec; no geometry stored today)*
- Q `GetSessionById` -- one session
- Q `ListSessionsByProject` -- sessions in a project
- Q `SearchSessions` -- fuzzy find sessions by name (sqlite-side) *(new)*
- Q `GetLaunchSpec` -- a session's launch spec (the recipe + placements)
- Q `GetPanelTree` -- a session's panel-tree geometry *(new)*

**Surface** (D9 side-effecting unless noted)
- C `SpawnSurface` -- add a surface to a session: persist `pending` + spawn via the runtime port (absorbs both the mint-placement and explicit-placement creation paths)
- C `StopSurface` -- kill the process inside a surface (PTY SIGKILL via the daemon); surface -> `idle`, record kept (resumable)
- C `CloseSurface` -- remove a surface from a session and kill its runtime proxy (deletes the record; absorbs both the by-id and by-(session,surface) removal paths)
- C `ReconcileSurfaces` -- boot reconciler (D9): converge daemon PTYs to desired rows via the daemon
  `List` frame -- desired-but-not-running -> respawn/mark failed; running-but-no-row -> kill. Does **not**
  attach proxies (attach is lazy/UI-driven via `AttachSurface`)
- C `DetachSurface` -- drop the proxy stream; the PTY keeps running in the daemon (bus command)
- io `SendSurfaceInput` -- send keystroke bytes to the surface's PTY (off-bus, never logged)
- io `ResizeSurface` -- resize the surface's PTY
- io `AttachSurface` -- connect the proxy stream to a running daemon PTY (= the runtime `resume` of one;
  lazy, per surface on view mount/revisit when its Channel is registered -- there is no eager boot attach-all)
- Q `GetSurfaceById` -- one surface
- Q `FindSurfaceByPlacement` -- surface bound to a session + placement
- Q `ListResumableSurfaces` -- surfaces eligible for resume
- Q `ListSurfacesBySession` -- a session's surfaces
- (status is an internal repo `update_status` write inside `SpawnSurface`/reconcile, not a command)

**Command library** (`Prebuilt` commands are immutable -- rename/edit/discard rejected)
- C `NewCommand` -- add a custom library command
- C `RenameCommand` -- rename a custom command
- C `EditCommand` -- change a custom command's `cli`/`args`/`env` *(new)*
- C `DuplicateCommand` -- clone a command (e.g. prebuilt -> editable custom) *(new)*
- C `PinCommand` / `UnpinCommand` -- toggle the `pinned` flag (favorite) *(new)*
- C `DiscardCommand` -- delete a custom command
- C `SeedCommands` -- seed the prebuilt commands (idempotent, boot)
- Q `GetCommandById` -- one library command
- Q `ListCommands` -- all library commands (optionally by origin)

**Launch template** (project-bound saved spec, sqlite -- `LaunchTemplate { project_id, spec }`)
- C `NewLaunchTemplate` -- create a project's saved launch spec
- C `ApplyTemplateSpec` -- replace a project's saved launch spec
- C `DiscardLaunchTemplate` -- delete a project's saved launch spec
- Q `GetLaunchTemplateById` -- one launch template
- Q `ListLaunchTemplatesByProject` -- a project's saved launch templates

**Template library** (portable bundles, config/fs -- the `<templates>/` library picked at session creation; prebuilt + custom; prebuilt immutable)
- C `ImportTemplate` -- add a custom template bundle *(new)*
- C `ExportTemplate` -- export a template bundle *(new)*
- C `DiscardTemplate` -- remove a custom template (prebuilt rejected) *(new)*
- C `PinTemplate` / `UnpinTemplate` -- toggle the `pinned` flag *(new)*
- Q `ListTemplates` -- the library (prebuilt + custom) *(new)*
- Q `GetTemplateById` -- one library template *(new)*

**Notification** *(`NotificationRecord` gains a `read` flag and a `snooze_until` timestamp)*
- C `RecordNotification` -- post a notification record
- C `MarkNotificationRead` -- mark one notification read *(new)*
- C `MarkAllNotificationsRead` -- mark every notification read *(new)*
- C `SnoozeNotification` -- defer a notification until a later time (`snooze_until`) *(new)*
- C `DisregardNotification` -- dismiss/delete a single notification *(new)*
- C `DisregardAllNotifications` -- dismiss/clear all notifications *(new)*
- C `PruneNotifications` -- retention cap: keep the most recent N (internal)
- Q `ListNotifications` -- recent notifications (limited)
- Q `ListUnreadNotifications` -- unread notifications *(new)*
- Q `CountUnreadNotifications` -- unread badge count *(new)*

**Setting** (config, via `shared::fs`)
- C `ApplySetting` -- set/override a setting value at a scope
- C `ResetSetting` -- clear an override at a scope (revert to inherited/default) *(new)*
- C `ReloadConfig` -- re-read all user-config (settings, active profile, theme, keybindings) from disk (pick up external edits) *(new)*
- Q `GetSetting` -- a raw setting value at a scope
- Q `ListSettings` -- overrides at a scope
- Q `ResolveSetting` -- effective value for a project after the cascade
- Q `ResolveSettings` -- the full effective settings map for a project *(new)*

**Profile** (config bundle, via `shared::fs`; drives the cascade)
- C `NewProfile` -- create a profile
- C `RenameProfile` -- rename a profile
- C `DuplicateProfile` -- copy a profile under a new name *(new)*
- C `DiscardProfile` -- delete a profile
- C `ActivateProfile` -- switch the active profile *(new)*
- C `ImportProfile` -- import a shared profile bundle *(new)*
- C `ExportProfile` -- export a profile bundle *(new)*
- Q `ListProfiles` -- all profiles
- Q `GetActiveProfile` -- the currently active profile

**Theme** (config/fs; prebuilt + custom, like templates)
- C `ActivateTheme` -- set the active theme
- C `ImportTheme` -- add a custom theme bundle
- C `DiscardTheme` -- remove a custom theme
- C `ExportTheme` -- export a theme bundle
- Q `ListThemes` -- available themes (prebuilt + custom)
- Q `GetActiveTheme` -- the currently active theme

**Keybinding** (config/fs; the keymap)
- C `RebindKey` -- set/override a binding (action -> chord)
- C `ResetKeybinding` -- revert one binding to its default
- C `ResetKeybindings` -- revert the whole keymap to defaults
- Q `ListKeybindings` -- the effective keymap
- Q `ResolveKeybinding` -- the chord(s) bound to an action (and inverse)

### "Attach/detach" is overloaded -- three independent axes

The word "attach/detach" names three different things; keep them separate:
- **process** (running vs killed): `SpawnSurface` / `StopSurface` -- domain + runtime.
- **proxy stream** (orchestrator bridging the daemon PTY vs not): `attach` connects the stream -- an
  `app` direct fn -> infra port, skipping the bus (no command object); `DetachSurface` drops it and is a
  regular bus command. Either way the PTY keeps running; this is the proxy, not the process.
- **window placement** (surface popped into its own OS window vs the parent's panel tree): **UI/chrome**,
  persisted in the frontend-local store (`pref`/`StoreState`), **not** an orchestrator command. The
  session's logical panel tree (splits/tabs) stays domain (`ArrangePanels`); which OS window renders it
  does not.

### Surface I/O channel -- input/resize/attach skip the bus, not `app/`

Every surface op still goes through `app/` (the host never reaches `infra` directly); the split is only
whether it goes through the **bus**:
- **bus -> `app` command -> infra** (persist + coordinate, CQS command objects): `SpawnSurface`,
  `StopSurface`, `CloseSurface`, `Stop{Session,Project,Workspace}Surfaces`, `ResumeAllSurfaces`.
- **`app` direct function -> infra `SurfaceRuntime` port, skipping the bus** (pure pass-through, no
  persistence, no rule): `input`, `resize`, `attach` (connect proxy stream). These are plain `app`
  functions the host calls directly -- **not** CQS command objects, so no `bus.execute`, no command
  object, no per-keystroke span/telemetry -- that forward to the runtime port. (`detach` is a regular bus
  command -- it's a deliberate, infrequent op, fine to dispatch and log.)

So `input`/`resize`/`attach` are **not** CQS commands and do not go through the bus, but they stay inside
`app/` for layering. They are the `SurfaceRuntime` **I/O channel**: a thin, high-frequency pass-through
(renderer -> `app` -> orchestrator proxy -> daemon -> PTY). Reasons:
- **Performance** -- a tracing span + command object per keystroke is absurd overhead.
- **Security/privacy** -- the bus logs every operation; **keystrokes must never be logged** (a typed
  password landing in `*.log`/OTel export is a real leak). Keeping input off the bus keeps it out of the
  telemetry path; the I/O channel **never logs payloads** (resize dimensions are fine; raw input bytes
  never). This is a hard telemetry-redaction rule (see D8).
- **Transport** -- the path is entirely local (tauri IPC in-process; daemon over a `0600` unix socket),
  so there is no network hop to intercept; remote interception is not in the threat model.

Input cannot be UI-only (the PTY lives in the daemon), but it is a runtime stream, not a domain command.

**The client never holds `SurfaceRuntime`; it holds a stream handle.** Two seams:
- **Output (PTY -> renderer):** the runtime pushes frames to a `SurfaceEventSink` port (defined in
  `infra`, pushed by the runtime adapter). The **tauri layer implements the sink** by writing to a
  per-surface `tauri::ipc::Channel<Vec<u8>>` -- the renderer creates the Channel and registers it
  (a thin tauri command at spawn/attach, keyed by `surface_id`); status/exit go out as tauri events.
  Channels are Tauri's ordered, low-overhead Rust->JS stream. A future web transport implements the same
  `SurfaceEventSink` with SSE/WebSocket -- same seam.
- **Input/resize (renderer -> PTY):** thin tauri commands that write straight to the `SurfaceRuntime`
  port, **bypassing the CQS bus** (no command object, no telemetry, never logged).

So `SurfaceRuntime` and the daemon socket stay server-side; the renderer's only handles are a `Channel`
(output) plus the input/resize endpoints.

**Starting the stream = register a `Channel` + `AttachSurface`.** The renderer registers a per-surface
`Channel`, then `AttachSurface` (io) subscribes the proxy to the daemon PTY and binds it to that Channel;
the call returns fast and output frames then flow asynchronously through the Channel (via the
`SurfaceEventSink`). Without a registered Channel there is nowhere to stream. `DetachSurface` (bus
command) tears the proxy down -- the Channel goes quiet while the PTY keeps running.

### Out of CQS scope (host/shell, not orchestrator domain)

UI window attach/detach (pop a surface to its own window / re-dock it) lives in the frontend-local store,
not the orchestrator. These tauri commands likewise stay host/transport concerns, not domain
command/query objects: `daemon_connect`/
`daemon_send`/`daemon_disconnect`/`daemon_ensure` (daemon bridge), `window_open`/`window_focus`/
`window_close` (window host), `log_forward`/`list_log_files`/`file_read`/`file_size` (log/file host),
`pref_get`/`pref_set`/`registry_*` (frontend-local store), `orchestrator_status`/`service_health`
(health/status), `command_center_set_leader` (UI). They are not part of this change's domain layer.

## Errors & OTel-ready logging

### D8 -- One error registry, OTel-ready, exported to `*.log` (no OTel crates)

All errors live in one enum in `shared/errors.rs` (the registry). The stable telemetry code is declared
**on each variant** via a second attribute, `#[error_code("...")]`, read by a small `#[derive(ErrorCode)]`
that generates `code()`. `Display`, `#[from]`, `#[source]`, and `transparent` stay with `thiserror` --
the `ErrorCode` derive only reads `#[error_code]` and emits the `code()` match; it does not touch
formatting. A variant missing `#[error_code]` is a compile error, so the registry stays complete. The
derive lives in a small core-owned proc-macro crate (`crates/custom-macro`). There is **no
`category`/`level`** -- every `shared::Error` logs at `ERROR` (a returned `Err` here is a genuine
operation failure; expected absence is `Ok(None)`, not an error), so the only metadata a variant carries
is its `code`.

```rust
// shared/errors.rs
#[derive(Debug, thiserror::Error, ErrorCode)]
pub enum Error {
    #[error("workspace not found: {0}")]
    #[error_code("workspace.not_found")]
    WorkspaceNotFound(WorkspaceId),

    #[error("the Default workspace cannot be deleted")]
    #[error_code("workspace.is_default")]
    WorkspaceIsDefault,

    #[error("invalid {field}: {reason}")]
    #[error_code("validation")]
    Validation { field: &'static str, reason: String },

    // infra — Display/source chain via thiserror; code via #[error_code]
    #[error(transparent)] #[error_code("db.error")]    Db(#[from] sqlx::Error),
    #[error(transparent)] #[error_code("io.error")]    Io(#[from] std::io::Error),
    #[error(transparent)] #[error_code("serde.error")] Serde(#[from] serde_json::Error),
}
// #[derive(ErrorCode)] generates: pub fn code(&self) -> &'static str { match … }  (missing attr = compile error)
```

Rules that make it OTel-ready without any OTel dependency:

- **`code()` is stable and low-cardinality** -- `"db.error"`, never the id. Ids live in the message and on
  the span, so they never become a metric label. `code` is explicit (declared per variant via
  `#[error_code]`, not derived from the variant name) so a rename can't silently break the telemetry
  contract.
- **Source chain preserved** -- `#[from] sqlx::Error`/`io::Error`/`serde_json::Error` (replacing the old
  stringly `Persistence(String)`), so the real cause is in `source()`.
- **Every error logs at `ERROR`** -- a returned `Err` is a real operation failure; outcomes that are
  *expected* (e.g. a missing row on a read) are `Ok(None)`, not errors, so there is no WARN tier and no
  `level`/`category` to maintain.
- **Never log secret/keystroke payloads** -- surface input bytes are never captured by spans, events, or
  metrics (they are off the bus entirely; see the Surface I/O channel). A typed password must never reach
  `*.log` or an OTel export.

The bus is the single instrumentation point: a span per operation plus one structured `ERROR` event with
OTel-named fields (`error.type` = `code()`, `exception.message`, `source`, `trace_id`):

```rust
let span = tracing::info_span!("command", action = C::NAME, trace_id = %trace_id);
c.handle(&self.cx).instrument(span).await.inspect_err(|e| {
    tracing::error!(error.type = e.code(), exception.message = %e, source = ?e.source());
})
```

Sink: structured **JSON lines** via `tracing-subscriber` (`json` + `env-filter`) + `tracing-appender`
(rolling file) to `*.log`. No `opentelemetry*` and no metrics crate now. Because `tracing`'s span/event/
field model and these field names map 1:1 to OTel, adding a `tracing-opentelemetry` + OTLP layer later is
purely additive -- the instrumentation is unchanged. A line in the log file:

```json
{ "timestamp": "2026-06-21T10:14:02.391Z", "level": "ERROR",
  "fields": { "error.type": "workspace.not_found",
              "exception.message": "workspace not found: ws_9f3c", "source": null },
  "span": { "name": "command", "action": "rename_workspace", "trace_id": "b1e7…" } }
```

## Test strategy

Guiding rule (TESTING_GUIDELINES): **tests assert behavior and contracts, never internals.** A refactor
that only changes internals should break **zero** behavior tests. So when a test *does* break here, it
was coupled to an implementation detail -- it is **rewritten to capture the behavior**, not patched to
compile and not preserved as-is.

The pyramid for this change:

- **Unit (TDD, red-first per spec scenario).** Entity rules (rename -> `title_source = Custom`, the
  `is_default`/`is_unfiled` guards, the cascade policy); `shared` contracts (`pagination` cursor
  behavior, the `Kv` round-trip/TTL against both `SqliteKv` and `MemoryKv`); `Error::code()`
  low-cardinality (and a variant without `#[error_code]` fails to compile). Pure and fast, no storage.
- **Integration.** Infra repos against a `:memory:` `SqlitePool` (migrations applied): round-trip,
  `list` by parent + pagination, update, delete. App command/query handlers via the `Bus` over a
  `:memory:` `Ctx`: load -> entity rule -> persist, archive cascade, workspace-delete-reassign
  (Default-delete rejected). The `command_contract` test (real `generate_context!()` + `tauri://localhost`)
  over a `:memory:` `Ctx`.
- **e2e.** The existing tauri-webdriver / shared-app suite is the **behavior-preservation net**: because
  IPC, the dynamic ACL, the wire protocol, and runtime behavior are unchanged, it must run **green,
  unchanged**. That is the primary evidence the refactor changed nothing observable.

Handling tests that break:

- Tests coupled to deleted internals (`Storage::in_memory`, the `MemoryBackend`, the `store/` wrappers,
  `OrchestratorError` variants) are **rewritten to behavior-level assertions** over the new path (a
  `:memory:` `Ctx`/pool and `shared::Error` codes), not patched to keep compiling.
- On-disk-format assertions (`infra/fs/tests.rs` asserting the slug-tree) test a deleted implementation,
  not behavior -- they are **dropped**; the behavior they implied ("a child belongs to its parent", "a
  rename persists") is re-expressed as repo/handler assertions over sqlite.
- The cross-backend parity scenario is dropped (one persistence path).

Gate: `bun run verify` (format:check + check-types + lint + test -- unit+integration; **e2e excluded**) green,
**then** `bun run e2e` (the tauri-webdriver / shared-app suite) passing unchanged as the behavior net. Both
must be green; CI runs the full suite including e2e.

## Risks

- **On-disk format break** (slug-tree -> sqlite), no migration. Accepted: pre-v1, no released users.
- **New dependency** (`sqlx`) and a `sqlx` migration/schema for the domain tables. Mitigated by sqlx's
  compile-time query checking catching schema drift at build time.
- **Wide caller churn.** Many hosts/internals move to the bus at once. Mitigated by the frozen IPC
  surface and the retained behavior suite.
- **Losing human-readable/git-navigable domain dirs** (an ADR-0023 property). Accepted; user-config stays
  file-based.
- **Side effects + persistence aren't truly atomic** (D9). A side effect (PTY spawn, process launch) runs
  outside the DB transaction, so a crash between persisting intent and recording the outcome leaves a
  `pending`/`failed` row. Mitigated by reconciliation (`ReconcileSurfaces`): the persisted desired state is
  the source of truth and the runtime is driven to match on boot. This is accepted as the crash-safe model;
  the lock is never held across the effect.

## Migration

None at runtime for IPC/ACL/wire. Domain on-disk data is abandoned (slug-tree dropped). The change is
otherwise schema, call-site, and test code.

## Open Questions

- `async fn` in traits natively vs `async-trait`/`trait_variant` -- decided at wiring time by whether the
  tokio runtime requires `Send` handler futures; does not affect the layer design.

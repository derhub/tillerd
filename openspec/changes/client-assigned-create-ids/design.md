## Context

The orchestrator core is a CQS layer: `Command<Cx>::handle` returns `Result<()>` (mutations carry no data), `Query<Cx>` returns `Out`. The desktop tauri transport (`apps/desktop/src-tauri/src/transport`) wraps each core command/query in a `#[tauri::command]` shim through three declarative macros: `transport_command!` (execute, return `()`), `transport_query!` (query, map to wire DTO), and `transport_create!` (mint id → execute → read back by id → map to DTO).

Four create operations exist. `workspace_create` already uses `transport_create!`: the caller mints a `WorkspaceId`, `NewWorkspaceCmd { id, name }` persists under it, and `GetWorkspaceById { id }` reads it back. The other three — `project_create`, `session_create`, `command_create` — mint their id *inside* the core command (`new_id()` for project, `SessionId::mint()` for session, internal for command), so the transport cannot read back by a known id. Instead each hand-written shim snapshots the id list before the create, re-lists after, and finds the row absent from the snapshot. This is ~140 lines, races under concurrent creates, and is the only reason these three shims are hand-written rather than `transport_create!` listings.

Separately, the full tauri handler list is duplicated: `collect_transport!` (production, in `transport/macros.rs`) and a hand-copied `generate_handler![...]` in `command_contract.rs` (the test). They differ by exactly one entry — `daemon_connect`, whose concrete `AppHandle<Wry>` cannot register on the test's `MockRuntime` — and must otherwise be kept in sync by hand.

Constraints: the orchestrator is the storage-de-abstraction frozen core; this change alters the create-command contract there, which is why it goes through a proposal. CQS purity (`handle -> Result<()>`) is held; only the id's origin moves.

## Goals / Non-Goals

**Goals:**

- All four aggregate create commands take a caller-assigned id, matching `NewWorkspaceCmd`.
- `project_create` and `command_create` collapse to `transport_create!` listings; `session_create` drops the list-diff (keeps its non-fatal `LaunchSession` tail).
- The two handler lists become one parameterized `collect_transport!`, so they cannot drift.
- Remove `new_id()` and the per-aggregate internal mint paths.

**Non-Goals:**

- No change to the renderer-facing IPC wire (command names, argument shapes, response JSON unchanged). The transport mints the id (`Uuid::new_v4`); the renderer is unaffected.
- Not implementing the `open_session` app-layer use case named in app-use-case-layer (pre-existing naming drift between that spec and the code; out of scope).
- Not touching the surviving `transport_command!` / `transport_query!` macros — they earn their keep and are not in scope.
- Renderer-minted ids (id flowing over the wire) deferred — see Open Questions.

## Decisions

**D1 — Caller mints the id at the transport, not the renderer.** Each create shim constructs `XId::new(Uuid::new_v4().to_string())` and passes it into the command and the read-back query. Alternative: have the renderer mint a UUID and send it over IPC (true end-to-end client identity, full cross-boundary idempotency). Rejected for now because it changes every create's wire shape and the SDK + every renderer call site; the transport-mint satisfies the CQS principle for the core (the transport is the core's client) with zero wire churn. Renderer-mint stays open as a later, separable step.

**D2 — Add an explicit `id` field to each create command; keep `handle -> Result<()>`.** `NewProjectCmd`, `NewSessionCmd`, `NewCommandCmd` gain `pub id: XId` and use it where they previously called the internal mint. CQS purity is preserved — the command still returns nothing. Alternative: change `Command::handle` to return the new id. Rejected: it breaks the `Command` trait contract and the `Bus::execute` signature for all commands to serve only creates, and contradicts the documented "a mutation returns no data" rule in `shared/cqs.rs`.

**D3 — Only identity is caller-assigned; derived fields stay in the command.** `created_at` (`now_iso()`), project `infer_name`, and template-spec resolution remain inside the command. The caller supplies an opaque id and nothing else. This keeps time and derivation as server concerns and is what makes the move principled rather than leaking server logic to the caller.

**D4 — `project_create` and `command_create` become `transport_create!`; `session_create` stays hand-written but minimal.** `session_create` chains a non-fatal `LaunchSession` after the create — a tail the `transport_create!` macro does not model — so it stays a hand-written `#[tauri::command]`, but reduces to mint → execute `NewSessionCmd` → `GetSessionById` → fire-and-forget `LaunchSession`. Alternative: extend `transport_create!` with an optional post-create action. Rejected: one special case does not justify growing the macro's surface.

**D5 — Parameterize `collect_transport!` with the runtime-specific commands.** `generate_handler!` is a proc-macro that needs literal idents at expansion time, so a shared list cannot be spliced into two separate `generate_handler!` calls. Instead `collect_transport!` takes the differing commands as macro arguments and emits the whole `generate_handler!` itself: `collect_transport!($crate::bridge::daemon_connect)` in `lib.rs`, `collect_transport!()` in the contract test. One canonical list; the divergence is a single documented argument. The per-command arg-shape `cases` vec in the contract test is left as-is — each entry needs a hand-written representative payload and cannot be derived.

**D6 — One translation point: the handler turns the input DTO into a domain entity; the repository speaks only entities.** The correct layering is `input DTO (app) → command handler builds the Entity → repository persists the Entity`. The handler is the single place that validates, applies defaults, and constructs value objects; the persistence layer knows only entities and value objects, never an input/`New*`/command type.

The current code violates this in two ways:
- **Input types leak into infra.** `infra/command.rs` does `use crate::entities::command::NewCommand` and `CommandRepo::create(cmd: &NewCommand)`; `workspace`, `launch_template`, and `surface_repo` repos do the same with their `New*`. The persistence layer knows the *shape of a create request* — an app concern leaking down.
- **Defaults and minting live in infra.** `CommandRepo::create` mints the id and `ProjectRepo::create` hardcodes `sort_order: 0` — domain decisions made in the wrong layer.

Outcome for the seven `New*`:

| `New*` | Today | After |
| --- | --- | --- |
| `NewProject` | DTO in `entities/`, command nests it | **deleted**; fields inline on the flat command; handler builds `Project` |
| `NewSession` | DTO in `entities/`, tuple-wrapped | **deleted**; fields inline on the flat command; handler builds `Session` (already the pattern) |
| `NewTemplate` | DTO in `entities/` | **deleted**; fields inline on the import command; handler builds `Template` |
| `NewWorkspace` | DTO in `entities/`, repo takes it | **deleted**; handler builds `Workspace`; `WorkspaceRepo::create(&Workspace)` |
| `NewCommand` (draft) | DTO in `entities/`, repo takes it | **deleted**; handler builds `Command`; `CommandRepo::create(&Command)` |
| `NewLaunchTemplate` | DTO in `entities/`, repo takes it | **deleted**; handler builds `LaunchTemplate`; repo takes the entity |
| `NewSurface` | DTO in `entities/`, repo takes it | **deleted**; handler builds `Surface`; repo takes the entity |

So every `New*` is *deleted*, not relocated — there is no persistence-write DTO and no infra-side draft. The create command (app) carries the flat input fields plus the caller-assigned id; its `handle` constructs the full entity (id, defaults, value objects, derived name) and calls `Repo::create(&Entity)`. Repositories converge on `create(exec, &Entity) -> Result<()>` — `SessionRepo::create(&Session)` already has this shape and is the reference. `entities/` is left with only aggregates, value objects, ids, and enums; `infra/` imports only those.

This also dissolves the `entities::command::NewCommand` / `app::command::NewCommand` name collision: the draft is deleted, so `NewCommand` exists only as the app command, and the `NewCommandDraft` alias is removed.

Alternative considered: keep a `New*` write struct beside each repo (a "parameter object"). Rejected — it still lets the persistence layer reason about a non-entity create shape, and it keeps default/derivation logic ambiguous between handler and repo. Funnelling through the entity gives one translation point and one thing infra knows.

Naming: the command absorbs the old DTO name (e.g. the command is `NewProject`, matching the existing `app::command::NewCommand`), collapsing the two-type duplication. Per-aggregate `*Cmd` suffixes can stay if preferred — naming is a detail, the layering is the point.

This relocation is a structural concern distinct from D1–D5 (placement, not behavior) but is folded in because it edits the same `New*` types and create paths; it can be split into its own change if a smaller diff is preferred.

**D7 — Layer enforcement is delivered by the `arch-rule-enforcement` change, not here.** This change establishes the `entities → infra → app` boundary; the mechanical, CI-gating check that locks it (ast-grep rules + a blocking scan step) is reusable infrastructure and lives in the separate `arch-rule-enforcement` change, which seeds the layer rules (`infra ↛ app`, `entities ↛ app/infra`) and the sqlx-macro guard. Sequencing: that harness lands first; this change's layering then stays green against it. The `domain-model-boundary` capability here states only that the boundary exists; "enforced automatically" is owned by `architecture-rule-enforcement`.

## Worked examples

### Example A — command-library (the leak: infra knows an input type)

Before — the draft lives in `entities/`, **`infra` imports it**, and the repo both mints the id and is the only one that knows the create shape:

```rust
// entities/command.rs — an input-shaped DTO sitting in the domain model
#[derive(Debug, Clone)]
pub struct NewCommand { pub name: String, pub origin: CommandOrigin, pub cli: String, pub args: Vec<String>, pub env: HashMap<String, String> }

// infra/command.rs — LEAK: persistence layer imports a create-input type
use crate::entities::command::NewCommand;
pub async fn create<'e>(exec: impl SqliteExecutor<'e>, cmd: &NewCommand) -> Result<CommandId> {
    let id = CommandId::mint();                 // <- minting (a domain decision) in infra
    // ...
    Ok(id)
}

// app/command/new_command.rs — command builds the draft, alias dodges the name collision
use crate::entities::command::{CommandOrigin, NewCommand as NewCommandDraft};
pub struct NewCommand { pub name: String, pub cli: String, pub args: Vec<String>, pub env: HashMap<String, String> }
impl BusCommand<Ctx> for NewCommand {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let draft = NewCommandDraft { name: self.name.clone(), origin: CommandOrigin::Custom, /* ... */ };
        CommandRepo::create(cx.db(), &draft).await?;
        Ok(())
    }
}
```

After — the handler builds the `Command` entity; `infra` imports only `Command`; the draft is deleted:

```rust
// app/command/new_command.rs — input DTO (flat, caller-assigned id) + the one translation point
use crate::entities::command::{Command, CommandOrigin};

pub struct NewCommand {
    pub id: CommandId,                           // <- caller-assigned
    pub name: String, pub cli: String, pub args: Vec<String>, pub env: HashMap<String, String>,
}
impl BusCommand<Ctx> for NewCommand {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        // input -> entity: defaults and value objects are decided HERE, in the app layer
        let command = Command {
            id: self.id.clone(),
            name: self.name.clone(),
            origin: CommandOrigin::Custom,       // default
            cli: self.cli.clone(),
            args: self.args.clone(),
            env: self.env.clone(),
            pinned: false,                       // default
        };
        CommandRepo::create(cx.db(), &command).await
    }
}

// infra/command.rs — knows ONLY the Command entity; no NewCommand, no mint, no defaults
use crate::entities::command::Command;
pub async fn create<'e>(exec: impl SqliteExecutor<'e>, cmd: &Command) -> Result<()> {
    let args_json = serde_json::to_string(&cmd.args)?;
    let env_json = serde_json::to_string(&cmd.env)?;
    // ... bind cmd.id / cmd.name / cmd.origin / cmd.cli / args_json / env_json / cmd.pinned ...
    Ok(())
}
```

`infra/command.rs` now imports `Command`, not `NewCommand` — the leak is gone. The transport shim becomes a `transport_create!` listing: mint `CommandId`, execute `NewCommand`, read back `GetCommandById`.

### Example B — project (the milder leak: flat args + a default in the repo)

`ProjectRepo::create` today takes flat input fields and hardcodes `sort_order: 0` — input shape and a default both in infra. After, the handler builds the `Project` entity and the repo takes `&Project`.

Before:

```rust
// entities/project.rs
/// Parameters for creating a new project.
#[derive(Debug, Clone)]
pub struct NewProject { pub source_kind: SourceKind, pub root_path: Option<String>, pub name: Option<String>, pub workspace_id: Option<WorkspaceId> }

// app/project/new_project_cmd.rs
use crate::entities::project::NewProject;
pub struct NewProjectCmd { pub params: NewProject }     // <- needless nesting
impl Command<Ctx> for NewProjectCmd {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = new_id();
        let workspace_id = self.params.workspace_id.clone().unwrap_or_else(WorkspaceId::default_id);
        let name = infer_name(&self.params);
        ProjectRepo::create(cx.db(), &id, &workspace_id, &name,
                            self.params.source_kind, self.params.root_path.as_deref(), 0).await?;  // <- 0 = default in infra
        Ok(())
    }
}

// infra/project.rs — flat input args, sort_order default baked in
pub async fn create<'e>(exec, id: &ProjectId, workspace_id: &WorkspaceId, name: &str, source_kind: SourceKind, root_path: Option<&str>, sort_order: u32) -> Result<()> { ... }
```

After — `NewProject` deleted; flat input command; handler builds the entity (all defaults here); repo takes `&Project`:

```rust
// app/project/new_project_cmd.rs — input DTO (flat) is the command
use crate::entities::project::{Project, ProjectStatus, SourceKind};

pub struct NewProject {
    pub id: ProjectId,                           // <- caller-assigned (new_id() deleted)
    pub source_kind: SourceKind,
    pub root_path: Option<String>,
    pub name: Option<String>,
    pub workspace_id: Option<WorkspaceId>,
}
impl Command<Ctx> for NewProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let project = Project {
            id: self.id.clone(),
            name: infer_name(self.name.as_deref(), self.root_path.as_deref()),       // derivation
            source_kind: self.source_kind,
            root_path: self.root_path.clone(),
            workspace_id: self.workspace_id.clone().unwrap_or_else(WorkspaceId::default_id),  // default
            sort_order: 0,                       // default — now in the handler, not the repo
            pinned: false,                       // default
            status: ProjectStatus::Active,       // default
        };
        ProjectRepo::create(cx.db(), &project).await
    }
}

// infra/project.rs — takes the entity, knows no input type, owns no defaults
pub async fn create<'e>(exec: impl SqliteExecutor<'e>, p: &Project) -> Result<()> { /* bind p.* */ Ok(()) }
```

Both examples land on the same shape: flat input command → handler builds the entity → `Repo::create(&Entity)`. `session` already does this (`SessionRepo::create(&Session)`); `workspace`, `launch_template`, and `surface` repos change the same way as `command`.

## Risks / Trade-offs

- **Frozen-core churn** → The id field touches every internal constructor of the three commands (`duplicate_command`, `duplicate_session`, their tests). Mitigation: the compiler flags every call site; the change is mechanical and arguably more correct (duplicate should own the new id).
- **Caller-supplied id collision** → A caller could pass a non-unique id. Mitigation: the repo create enforces the primary key and surfaces a typed error; `Uuid::new_v4` collision is not a practical risk.
- **`transport_create!` now has 4 uses incl. the read-back-or-error path** → If a create succeeds but the read-back returns `None`, the handler returns the `missing` error. This matches today's `workspace_create` behavior; no regression.
- **Handler-list macro arg is easy to mis-pass** → Passing `daemon_connect` to the test (which can't register it) would fail the build loudly, not silently. Acceptable — failure is a compile error.

## Migration Plan

1. Add the `id` field to the three core create commands; replace internal mint with the field; remove `new_id()`.
2. Update internal constructors (`duplicate_*`, tests) to mint and pass an id.
3. Rewrite `project_create` / `command_create` as `transport_create!`; trim `session_create` to mint + read-back + launch.
4. Parameterize `collect_transport!`; update `lib.rs` and replace the contract test's hand-copied list with `collect_transport!()`.
5. Run the command-contract test and the orchestrator create tests — both exercise the new path end to end.

No runtime data migration: ids are still opaque strings persisted to the same columns; only who generates them changes. Rollback is a straight revert (no persisted-shape change).

## Open Questions

- Renderer-minted ids: should the id eventually flow from the renderer over IPC for true end-to-end idempotency, accepting the SDK/wire change? Deferred, not blocking.

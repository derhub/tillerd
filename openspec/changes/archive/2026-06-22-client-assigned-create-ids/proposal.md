## Why

Three of the four create operations in the desktop transport (`project_create`, `session_create`, `command_create`) mint their entity id *inside* the orchestrator command, so the transport never learns the new id. To return the created record it snapshots the id list before the create, re-lists after, and diffs to find the new row — ~140 lines of fragile, race-prone boilerplate. `workspace_create` already avoids this by minting the id at the caller and reading back by id (`transport_create!`). The inconsistency is the root cause: CQS commands return `()` precisely because the caller is meant to already know the id. Aligning all create commands on caller-assigned identity removes the list-diff entirely and makes creates idempotent.

## What Changes

- **BREAKING (internal app API):** the four create commands that mint internally — `NewProjectCmd`, `NewSessionCmd` (mint in the command), `NewCommand`, `NewLaunchTemplateCmd` (mint in the repo) — take a caller-supplied id and stop minting, matching `NewWorkspaceCmd` and `NewProfile` (already caller-assigned). `handle()` keeps returning `Result<()>` — CQS purity is unchanged. Only *identity* moves to the caller; `created_at`, inferred name, and template-spec resolution stay server-side, built in the handler.
- **Layering fix:** each create handler builds the full domain entity (id, defaults, value objects) and calls `Repo::create(&Entity)`. Repositories that today take a `New*`/draft (`command`, `workspace`, `launch_template`, `surface`) or flat input fields with a baked-in default (`project`) change to accept the entity and return `()` — so the persistence layer imports only entities and value objects, never an input type. `SessionRepo::create(&Session)` already has this shape.
- Internal constructors of these commands (`duplicate_command`, `duplicate_session`, `duplicate_project`, and their tests) mint and pass an id.
- The hand-written list-diff shims `project_create` and `command_create` collapse to `transport_create!` listings; `session_create` keeps its non-fatal `LaunchSession` tail but drops the list-diff (mint → execute → read back by id → launch).
- The duplicated tauri handler list — `collect_transport!` (production) and the hand-copied list in `command_contract.rs` (test) — unifies into one parameterized `collect_transport!` so the two can no longer drift; the only difference (`daemon_connect`, unregisterable on the test `MockRuntime`) becomes a macro argument.
- `new_id()` and the per-aggregate internal mint paths are removed.
- **Layering cleanup (distinct but cohesive concern):** all seven `New*` input DTOs in `entities/` are **deleted**. The create command (app) carries the flat input fields; its handler builds the entity; the repo persists the entity. No draft survives in `entities/` or `infra/`. `entities/` is left holding only aggregates, value objects, ids, and enums. The `entities::command::NewCommand` / `app::command::NewCommand` name collision dissolves — the draft is gone, so `NewCommand` is only the app command.
- **Layer enforcement is out of scope here:** the mechanical, CI-gating check that locks `infra ↛ app` / `entities ↛ app·infra` is delivered by the separate `arch-rule-enforcement` change. This change establishes the boundary; that change guards it.

## Capabilities

### New Capabilities

- `client-assigned-identity`: every aggregate create command carries a caller-minted id; the create handler returns the entity by reading it back by that id rather than diffing a list snapshot; a create is idempotent on its id; CQS commands continue to return no data.
- `domain-model-boundary`: the `entities` module holds only the domain model (aggregates, value objects, ids, enums); command-input and persistence-write DTOs live in the layer that consumes them (the command for app-only inputs, the repository for persistence inputs), never in `entities`.

### Modified Capabilities

<!-- None. Renderer-facing IPC contracts (workspace-ipc, command-library) are unchanged: the
     transport mints the id and the wire request/response shapes stay byte-identical. The change
     is below that surface — the orchestrator command contract and the transport mechanism. -->

## Impact

- **Code (orchestrator core):** `app/project/new_project_cmd.rs`, `app/session/new_session_cmd.rs`, `app/command/new_command.rs`, `app/template/new_launch_template_cmd.rs` gain an id field and drop internal minting; `infra/command.rs` and `infra/launch_template.rs` `create` take the id; `app/project/common.rs` loses `new_id()`; internal callers (`duplicate_*`, tests) pass an id. Touches the storage-de-abstraction frozen core — hence this proposal.
- **Code (DTO deletion + repo signatures):** seven `New*` types are deleted from `entities/`; their fields move onto the flat create commands and each handler builds the entity; `ProjectRepo`/`CommandRepo`/`WorkspaceRepo`/`LaunchTemplateRepo`/`SurfaceRepo` `create` change to take `&Entity` and return `()`; `entities/mod.rs` re-exports and every `use crate::entities::*::New*` import are updated; internal callers (`duplicate_*`, seeding, tests) build entities.
- **Layer enforcement:** delivered by the `arch-rule-enforcement` change (ast-grep rules + blocking CI step), not by this change.
- **Code (desktop transport):** `transport/domain.rs` — `project_create`/`command_create` become `transport_create!`, `session_create` loses its list-diff; `transport/macros.rs` — `collect_transport!` gains a parameter; `command_contract.rs` — the hand-copied handler list is replaced by a `collect_transport!()` call.
- **Wire / SDK:** none. Renderer-facing command names, argument shapes, and response JSON are unchanged.
- **Behavior:** create operations no longer race under concurrent creates (read-back by exact id); creates become retry-safe.

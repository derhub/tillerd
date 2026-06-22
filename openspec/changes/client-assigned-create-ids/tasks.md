## 1. Core: caller-assigned id + handler-builds-entity

- [x] 1.1 Add `pub id: ProjectId` to the project create command; build the `Project` entity in `handle` (id + defaults: `sort_order: 0`, `pinned: false`, `status: Active`, `workspace_id` default); `ProjectRepo::create(&Project)`; delete `new_id()` from `app/project/common.rs`.
- [x] 1.2 Add `pub id: SessionId` to the session create command; use it in the existing `Session` build instead of `SessionId::mint()` (repo already takes `&Session`).
- [x] 1.3 Add `pub id: CommandId` to `NewCommand`; build the `Command` entity in `handle` (origin `Custom`, `pinned: false`); change `CommandRepo::create` to take `&Command`, return `()`, drop the internal `CommandId::mint()`.
- [x] 1.4 Add `pub id: LaunchTemplateId` to the launch-template create command; build the `LaunchTemplate` entity in `handle`; change `LaunchTemplateRepo::create` to take `&LaunchTemplate`, return `()`, drop the internal mint.
- [x] 1.5 Update internal constructors and tests (`duplicate_project`, `duplicate_session`, `duplicate_command`, create tests in `new_*_cmd.rs` / `search_*.rs` / `test_util.rs`) to mint an id and build the entity.

## 2. Transport: collapse the list-diff creates

- [x] 2.1 Rewrite `project_create` as a `transport_create!` listing (mint `ProjectId`, execute `NewProjectCmd`, read back `GetProjectById`).
- [x] 2.2 Rewrite `command_create` as a `transport_create!` listing (mint `CommandId`, execute `NewCommandCmd`, read back `GetCommandById`).
- [x] 2.3 Trim `session_create` to mint `SessionId` → execute `NewSessionCmd` → `GetSessionById` → fire-and-forget `LaunchSession`; remove the before/after list snapshot.

## 3. Transport: unify the handler list

- [x] 3.1 Parameterize `collect_transport!` to take the runtime-specific command(s) as macro args, with `daemon_connect` excluded from the base list.
- [x] 3.2 Update `lib.rs` to call `collect_transport!($crate::bridge::daemon_connect)`.
- [x] 3.3 Replace the hand-copied `generate_handler![...]` list in `command_contract.rs` with `collect_transport!()`; keep the `cases` arg-shape vec.

## 4. Delete `New*` input DTOs; repos take entities

- [x] 4.1 Delete all seven `New*` from `entities/`; inline their fields onto the flat create commands (drop the `NewSessionCmd` tuple wrapper; switch `infer_name` from `&NewProject` to its two fields).
- [x] 4.2 Change the remaining draft-taking repos to accept the entity: `WorkspaceRepo::create(&Workspace)`, `SurfaceRepo::create(&Surface)` (and `CommandRepo`/`LaunchTemplateRepo` from tasks 1.3/1.4). Remove every `use crate::entities::*::New*` from `infra/` — infra imports only entities and value objects.
- [x] 4.3 Drop the deleted types from `entities/mod.rs` re-exports; update all `use crate::entities::*::New*` import sites.

## 5. Verify

- [x] 5.1 Run the command-contract test, the orchestrator create/duplicate tests, and `cargo clippy --all-targets -- -D warnings`; fix everything until green. (Layer enforcement via ast-grep is delivered by the `arch-rule-enforcement` change.)

> Greening note: sections 4-5 ARE the `entities-no-input-dto` rule-greening (7 `New*` findings). The
> remaining seven ast-grep rules are greened by `storage-de-abstraction` Phase 6; the `arch-rule-enforcement`
> change was never created, so the `warning -> error` flip lives in Phase 6f there.

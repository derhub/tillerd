Each domain task: add a `transport_command!`/`transport_query!`/`transport_create!` shim per unexposed handler in `transport/domain.rs`, add the command ident to `collect_transport!`, add a `command_contract.rs` `cases` entry (representative arg JSON), and a `*View` `assert_keys` shape test for each new view type. Run the contract test green before moving on. Arg shapes are read off each handler's fields; do not invent.

## 0. Prerequisite gate

- [x] 0.1 Confirm `client-assigned-create-ids` has landed: `NewLaunchTemplateCmd` carries a caller `id` field, and `command_contract.rs` registers via `collect_transport!()` (not a hand-copied `generate_handler!`). Do not start until both hold.

## 1. Wire the template domain

- [x] 1.1 `app/mod.rs`: add `pub mod template;`. Confirm `cargo check -p tillerd-orchestrator` is clean.

## 2. Expose domain commands (per-domain shim batches)

- [x] 2.1 command (6): `RenameCommand`, `EditCommand`, `PinCommand`, `UnpinCommand`, `DuplicateCommand`, `SeedCommands`.
- [x] 2.2 notification (9): unread list/count queries + mark/disregard (single + all) + snooze + prune + record.
- [x] 2.3 project (7): `GetProjectById`, `SearchProjects` (queries); `RestoreProject`, `DuplicateProject`, `PinProject`, `UnpinProject`, `StopProjectSurfaces`.
- [x] 2.4 session (12): `ListAllSessions`, `GetSessionById`, `GetLaunchSpec`, `SearchSessions` (queries); `LaunchSession`, `ApplyLaunchSpec`, `MoveSession`, `DuplicateSession`, `PinSession`, `UnpinSession`, `RestoreSession`, `StopSessionSurfaces`.
- [x] 2.5 settings (24): profile (`GetActive`/`List`/`New`(create)/`Activate`/`Rename`/`Duplicate`/`Discard`/`Export`/`Import`), theme (`GetActive`/`List`/`Activate`/`Discard`/`Export`/`Import`), keybinding (`List`/`Rebind`/`Reset`/`ResetAll`/`Resolve`), `ResetSetting`, `ResolveSetting`, `ResolveSettings`, `ReloadConfig`.
- [x] 2.6 surface (6): `GetSurfaceById`, `ListSurfacesBySession`, `ListResumableSurfaces`, `FindSurfaceByPlacement` (queries); `StopSurface`, `ReconcileSurfaces`.
- [x] 2.7 template (12): `NewLaunchTemplateCmd` (create, via `transport_create!`), `ListLaunchTemplatesByProject`, `GetLaunchTemplateById`, `DiscardLaunchTemplate`, `ApplyTemplateSpec`, `ListTemplates`, `GetTemplateById`, `ImportTemplate`, `ExportTemplate`, `DiscardTemplate`, `PinTemplate`, `UnpinTemplate`.
- [x] 2.8 workspace (6): `GetWorkspaceById` (query); `ArchiveWorkspace`, `RestoreWorkspace`, `PinWorkspace`, `UnpinWorkspace`, `StopWorkspaceSurfaces`.

## 3. Residual cleanup

- [x] 3.1 Remove redundant `entities/mod.rs` + `infra/mod.rs` flat re-exports (internal code uses full paths) and any entity helper still unused after exposure, so `clippy -D warnings` is clean. Keep any item a newly-exposed handler now uses.

## 4. Verify gate

- [x] 4.1 Fix-all: `bun run verify` green (format, types, `clippy -D warnings`, tests — incl. the contract test enumerating all new commands + every `*View` shape test); `bun run e2e` green; `ast-grep scan`/`test` green.

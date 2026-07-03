# Tasks — prune-dead-code

## 1. UI transport chain

- [x] 1.1 Move the live `TauriCore`/`TauriChannelLike` types into `apps/ui/app/lib/transport/core.ts`;
  delete `framed.ts`, `tauri.ts`, `logger.ts`, `app-data.ts`, `terminal-bind.ts`, `bytes.ts` and
  their `.test.ts` files; shrink `transport/index.ts` to the live exports. rg-prove zero remaining
  references to each removed symbol.
- [x] 1.2 Delete `apps/ui/app/welcome/` + logo svgs; remove the `discardLegacyLayout()` shim from
  `usePanelTree.ts` (and its test coverage if any).

## 2. Dead TS packages

- [x] 2.1 Delete `packages/sdk` and `packages/logger` (both zero-consumer; logger was the sdk's only
  importer and is itself a node-only pino logger for the retired TS engine); drop the `@tillerd/sdk`
  dep from `apps/ui/package.json`; refresh the lockfile.
- [x] 2.2 Update stale references: `command_contract.rs` doc comments name the generated bindings;
  the openspec config packages list reflects the real workspace.

## 3. Desktop dead modules

- [x] 3.1 Delete `bridge.rs`; unwire `BridgeState`, the `collect_transport!(bridge::daemon_connect)`
  entry, `bridge::daemon_send`/`daemon_disconnect`, and the `DaemonLost` specta event from `lib.rs`.
- [x] 3.2 Delete `daemon_session.rs` and `gate_admin.rs` (+ their `mod` decls); confirm no test or
  bench references remain.
- [x] 3.3 Remove the `notification_record` command from `transport/notification.rs`, its
  registration, and its `command_contract.rs` case.

## 4. client-bindings barrel

- [x] 4.1 Remove the unused `dropById`/`reorderByIds`/`mergeById` exports (delete the functions if
  nothing else uses them; keep any that other module code calls internally).

## 5. Fix-all gate

- [x] 5.1 rg sweep: every removed symbol/command name has zero hits outside git history; then full
  `bun run verify`, `ast-grep scan` (0 errors), and the full e2e suite green.

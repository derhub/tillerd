# prune-dead-code

## Why

The architecture freeze audit (pre-UI-overhaul) found a dead subsystem and several orphaned
modules that compile, carry tests, and read as live surface. The old daemon byte-transport was
superseded end to end by the `command`/`query`/`subscribe`/`channel` transport (ADR-0041/0042/0043)
but its client half was never deleted: the UI transport chain, the SDK protocol/event types, and the
desktop `bridge.rs` commands have zero production callers. Freezing the architecture with a phantom
subsystem in place would mislead the UI overhaul and lock in tech debt. Pre-v1 breaking changes are
sanctioned; git history preserves everything removed.

## What Changes

- Remove the dead UI daemon-transport chain: `apps/ui/app/lib/transport/{framed,logger,app-data,terminal-bind,bytes}.ts`,
  the `TauriDaemonTransport` class (keep `core.ts` and the live `TauriCore`/`TauriChannelLike` types),
  their tests, and the barrel exports.
- Delete `packages/sdk` and `packages/logger` outright: the SDK's only importer was
  `packages/logger`, and `@tillerd/logger` itself has zero consumers — a node-only pino logger for
  the retired TS engine. Backend logging is Rust `tracing`; no TS package needs the `Logger` types.
  Drop the `@tillerd/sdk` dependency from `apps/ui`; update stale `@tillerd/sdk` comment references
  in `command_contract.rs` and the openspec config packages list.
- Remove the desktop daemon byte-bridge: `bridge.rs` (`daemon_connect`/`daemon_send`/`daemon_disconnect`,
  `BridgeState`, `DaemonLost` event), unwire from `lib.rs`. Daemon-death UX is covered by service health.
- Delete checked-in dead modules `daemon_session.rs` and `gate_admin.rs` (agent-session hook auth,
  deferred to 1.x with the agent surface; reintroduce then against the current transport).
- Remove the uncalled `notification_record` command from the desktop transport.
- Remove `apps/ui/app/welcome/` and its logo assets (unreferenced template scaffold).
- Remove the `discardLegacyLayout()` one-time migration shim from `usePanelTree.ts`.
- Remove unused `dropById`/`reorderByIds`/`mergeById` barrel exports from `@tillerd/client-bindings`.

## Capabilities

### New Capabilities

_None — this change only removes dead surface._

### Modified Capabilities

- `desktop-command-coverage`: the command contract is asserted against the generated bindings
  (`@tillerd/client-bindings`), not the retired `@tillerd/sdk` client.
- `observability-logging`: the TS core-package logging-library-agnostic requirement is removed with
  the retired TS packages (`@tillerd/sdk`, `@tillerd/engine`, `@tillerd/logger`); Rust `tracing`
  owns process logging.

## Impact

- `apps/ui`: transport chain deleted (~486 lines + tests), `welcome/` deleted, `usePanelTree` shim
  dropped, `@tillerd/sdk` dependency removed.
- `packages/sdk` + `packages/logger`: deleted (both zero-consumer).
- `apps/desktop/src-tauri`: `bridge.rs`, `daemon_session.rs`, `gate_admin.rs`, `notification_record`
  removed; `lib.rs` invoke-handler/specta wiring shrinks; `command_contract.rs` comments updated.
- `packages/client-bindings`: three dead barrel exports removed.
- No wire, daemon, or gate change: the pty daemon, gate, and their Rust clients are untouched
  (0.0.6 frozen boundaries not crossed; the gate is never purged).
- Breaking pre-v1: `@tillerd/sdk` package ceases to exist; importers are updated in the same change.

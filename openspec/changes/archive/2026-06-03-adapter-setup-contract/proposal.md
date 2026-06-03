## Why

Setup is currently expressed as a pure plan (`planHookInstall`/`planHookUninstall`) that a host
driver executes against a single settings value. That confines an adapter to one settings-file
transform and leaves the read/backup/write orchestration in the host. A forthcoming host CLI needs
to invoke an adapter's setup directly, and an adapter needs full control over its own install and
uninstall procedure — ordering, conditionals, more than one target — not just a value transform.

## What Changes

- Add a `defineSetup({ install, uninstall })` contract: an adapter declares its setup as two
  procedures the host invokes directly. The adapter owns the full install/uninstall flow and the
  agent-specific decisions (which file, which keys, idempotency); the host supplies a setup context
  carrying the notify command, the resolved agent-home, a logger, and a filesystem capability
  (`readText`, `writeAtomic`, `backup`, `exists`). The adapter computes the content and calls the
  capability for the generic mechanics — backup and atomic write are implemented once in the host
  and reused by every adapter, not reimplemented per adapter.
- Because the procedures touch only the injected capability and pure string operations, the adapter
  stays import-safe (zero host primitives at load and in every contract function) — the setup is a
  separate export from the engine-facing definition, not a separate I/O-bearing entry.
- **BREAKING**: Remove the pure-plan hook API from the engine-facing `AgentDefinition` — the
  `hookInstall` spec datum, `planHookInstall`, and `planHookUninstall` — and the host's
  `installHooks`/`uninstallHooks` plan-driver. The hook install/uninstall logic moves into the
  adapter's setup procedures with full procedural control.
- The composition root (server) no longer installs hooks at startup; it assumes setup is already
  complete. Setup is invoked by the installer (a forthcoming host CLI) through the new contract.
- The engine is unchanged: setup remains a precondition it assumes is already complete. Enable/
  disable state is out of scope.

## Capabilities

### New Capabilities

- `adapter-setup`: the `defineSetup` contract — an adapter supplies `install` and `uninstall`
  procedures that the host invokes directly to perform agent setup, with the adapter owning the
  full procedure and the host supplying only a setup context.

### Modified Capabilities

- `claude-code-agent`: the engine-facing `AgentDefinition` drops the declarative hook-install spec
  and the pure hook-planning functions; the adapter supplies its setup through the new
  `adapter-setup` contract instead, and the engine-facing module remains import-safe.

## Impact

- `@athing/sdk` — add the `defineSetup` contract (`SetupDefinition`, `SetupContext`, the `SetupFs`
  capability type); remove the hook-plan types (`HookInstallSpec`, `HookPlan`, `AgentSettings`) and
  the `hookInstall`/`planHookInstall`/`planHookUninstall` members from `AgentDefinition`.
- `@athing/adapter-claude-code` — supply the setup procedures (a separate `setup` export built via
  `defineSetup`) that compute content and call the injected `SetupFs`; remove the pure-plan hook
  functions from the engine-facing module.
- `@athing/platform-bun` — replace the `installHooks`/`uninstallHooks` plan-driver with the
  `SetupFs` capability implementation (read, atomic write, timestamped backup) and assembly of the
  setup context; keep notify-script preparation.
- `apps/server` — removes the startup hook auto-install and its notify-script preparation; the
  server assumes setup is already complete (keeps the CLI version check).
- Tests for the hook setup, the adapter index, and the host driver.

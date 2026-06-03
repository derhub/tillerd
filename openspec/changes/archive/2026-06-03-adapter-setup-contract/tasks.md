## 1. Contract (@athing/sdk)

- [x] 1.1 Add the setup contract: `SetupFs` capability (`readText`/`writeAtomic`/`backup`/`exists`),
      `SetupContext` (resolved notify command, resolved agent-home, logger, `fs: SetupFs`),
      `SetupDefinition` (`install(context)`/`uninstall(context)` async procedures), and the
      `defineSetup(def)` typed identity helper; export them
- [x] 1.2 Remove `hookInstall`, `planHookInstall`, `planHookUninstall` from `AgentDefinition` and
      the `HookInstallSpec`/`HookPlan`/`AgentSettings` types and their exports

## 2. Adapter setup (@athing/adapter-claude-code)

- [x] 2.1 Add a `setup` export (`src/setup.ts`) built via `defineSetup`; `install`/`uninstall`
      assemble the settings path from `context.agentHome`, read via `context.fs.readText`, compute
      the next content (reusing the hook constants: events, marker, matcher rule), and persist via
      `context.fs.backup` + `context.fs.writeAtomic` — no direct host primitives
- [x] 2.2 Keep install/uninstall idempotent (already-present install and already-absent uninstall
      make no change) inside the procedures
- [x] 2.3 Export `setup` as a sibling to `claudeCode`; ensure the engine-facing `index.ts` exposes
      no setup member and keeps zero host-primitive access
- [x] 2.4 Remove the pure-plan hook functions and `hookInstall` data from the engine-facing module

## 3. Host (@athing/platform-bun)

- [x] 3.1 Replace `installHooks`/`uninstallHooks` and the `hooks.ts` plan-driver with a `SetupFs`
      implementation (read, timestamped backup, temp+rename atomic write); keep notify-script
      preparation
- [x] 3.2 Provide assembly of the `SetupContext` (notify command, agent-home, logger, `SetupFs`)

## 4. Composition root (apps/server)

- [x] 4.1 Remove the startup hook auto-install (and notify-script preparation); the server assumes
      setup is already complete and keeps only the CLI version check. Setup is the installer's job.

## 5. Tests

- [x] 5.1 Test the adapter's `install`/`uninstall` against a fake in-memory `SetupFs`: install adds
      the events, uninstall removes them, both idempotent, backup invoked before write, unrelated
      settings preserved
- [x] 5.2 Update the adapter `index.test` to assert no `hookInstall`/`planHookInstall`/
      `planHookUninstall` members and that `setup` is a separate sibling export
- [x] 5.3 Update the import-safety test to assert both the engine-facing definition and the `setup`
      export touch no host primitive
- [x] 5.4 Replace the platform-bun host-driver test with a `SetupFs` test covering the real
      read/backup/atomic-write ordering and no-temp-file-left behavior

## 6. Verify

- [x] 6.1 `bun test` green across affected packages
- [x] 6.2 `openspec validate adapter-setup-contract` passes

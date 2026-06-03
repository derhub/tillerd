## 1. Contract (@athing/sdk)

- [x] 1.1 Remove `installHooks`/`uninstallHooks` from `AgentDefinition`; add the hook-install spec
      data shape (settings-file location, command template, event list, hook marker, matcher rule)
      and the pure planning function types `planHookInstall`/`planHookUninstall` returning
      `{ settings, changed, events }`
- [x] 1.2 Change `transcriptPath` signature to `transcriptPath(sessionId, cwd, agentHome): string`

## 2. Adapter pure functions (@athing/adapter-claude-code)

- [x] 2.1 Rewrite `hook-installer.ts` as pure `planHookInstall(currentSettings, notifyCommand)` and
      `planHookUninstall(currentSettings)` — no `node:fs`/`node:os`/`node:path` imports; move the
      idempotency ("already present / already absent → no change") into the plan
- [x] 2.2 Extract the hook-install spec data (settings path, event list, marker, matcher rule,
      command template) as adapter config
- [x] 2.3 Rewrite `transcript-path.ts` to take `agentHome` and build the path with pure string ops;
      remove `node:os`/`node:path`
- [x] 2.4 Update `index.ts` `claudeCode` definition to expose the hook-install spec data and pure
      functions; confirm the module imports zero host primitives at load

## 3. Host I/O (@athing/platform-bun)

- [x] 3.1 Add a host-side `installHooks`/`uninstallHooks` that read the settings file, call the
      adapter's pure plan, and on change back up and atomically write (temp + rename), reusing the
      existing backup/atomic-write behavior
- [x] 3.2 Resolve the agent-home location in the host and export it for startup injection

## 4. Engine threading (@athing/engine)

- [x] 4.1 Accept the caller-resolved agent-home as a startup value and thread it into the content
      path's call to `adapter.transcriptPath(sessionId, cwd, agentHome)`
- [x] 4.2 Update the `AgentDefinition` test doubles in `content.test.ts` and `proxy.test.ts` to the
      new contract (no install/uninstall methods; new `transcriptPath` arity)

## 5. Composition root (apps/server)

- [x] 5.1 Replace `claudeCode.installHooks(...)` with the host-side install call from the platform
      package
- [x] 5.2 Resolve and pass the agent-home startup value into the engine bootstrap

## 6. Tests

- [x] 6.1 Rewrite `hook-installer.test.ts` as pure-plan tests (plain objects in/out, install +
      uninstall + idempotency), no filesystem
- [x] 6.2 Add a host-side test covering settings read/backup/atomic-write ordering and no-change
      skip
- [x] 6.3 Update `transcript-path.test.ts` for the `agentHome` parameter and pure string assembly
- [x] 6.4 Update `index.test.ts` to assert the declarative spec + pure functions and the absence of
      install/uninstall methods
- [x] 6.5 Add a test asserting the adapter module imports with no host-primitive access
- [x] 6.6 Assert engine content resolution is unchanged given a substitute agent-home value

## 7. Verify

- [x] 7.1 `bun test` green across affected packages
- [x] 7.2 `openspec verify --change standalone-hook-installer` (or `/opsx:verify`) passes

## 8. Binary-resolution seam (close the remaining host import)

- [x] 8.1 Replace `resolveCommand()` on `AgentDefinition` with declarative `binaryResolution` spec
      data (override env var, binary name, common install locations)
- [x] 8.2 Delete the adapter's `resolve.ts`; add pure `binary-resolution.ts` data; drop the
      `node:child_process`/`node:fs` imports and module-load `process.env` read
- [x] 8.3 Add host `resolveAgentCommand(spec)` in `@athing/platform-bun` (override → login-shell
      PATH → common locations, `~` expanded); replace the old `resolveBinary`
- [x] 8.4 Thread the resolved command as an engine startup value (`EngineDeps.resolvedCommand` →
      proxy spawn frame) instead of calling `adapter.resolveCommand()`
- [x] 8.5 Composition root resolves the command via the host and passes it into the engine bootstrap
- [x] 8.6 Update test doubles/fixtures and `bootstrap.test`/`index.test`; strengthen the
      import-safety test to scan the whole adapter `src/`

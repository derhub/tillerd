## 0. Layout (post refactor-engine-io-ports)

- The runtime-based hook client now lives at `packages/platform-bun/src/notify.ts` (executed via
  `bun`), built into `$ATHING_DIR/notify.mjs` by the platform-bun `build` script. Host resolution
  is in `packages/platform-bun/src/ingress.ts` (`notifyScriptPath`, `notifyCommand`,
  `prepareNotifyScript`). The adapter keys hook install/uninstall on the marker `notify.mjs`
  (`packages/adapter-claude-code/src/hook-installer.ts`). This change replaces all of that with a
  committed, runtime-free shell client.

## 1. Standalone shell client committed in `bin/`

- [x] 1.1 `bin/` is currently fully gitignored (generated wrappers only). Un-ignore the source
      client: add `!athing-notify` to `bin/.gitignore`.
- [x] 1.2 Add `bin/athing-notify` (`#!/usr/bin/env bash`, no extension), committed as source.
      Behavior: read the lifecycle payload from stdin, forward it verbatim to `$ATHING_BRIDGE_URL`
      with headers `x-session-token: $ATHING_SESSION_TOKEN`, `x-session-id: $ATHING_SESSION_ID`,
      `content-type: application/json`.
- [x] 1.3 Support both bridge forms (mirroring current `notify.ts`): a value beginning with `/`
      delivers over the local control-channel socket (no network port); any other value is a URL.
- [x] 1.4 Fire-and-forget: bound runtime (short max time), discard output, suppress all errors,
      always `exit 0`. If `$ATHING_BRIDGE_URL` is unset/empty, exit 0 immediately.
- [x] 1.5 `chmod +x bin/athing-notify`; confirm committed executable.

## 2. Host resolves and points the hook command at the shell client

- [x] 2.1 Rewrite `notifyScriptPath()` in `packages/platform-bun/src/ingress.ts` to resolve the
      committed `bin/athing-notify` using the same fallback chain as `resolveDaemonBinary()` in
      `supervisor.ts`: `ATHING_NOTIFY_BIN` env, then `cwd()/bin/athing-notify`, then
      `import.meta.dir` module-relative `../../../../bin/athing-notify`.
- [x] 2.2 Change `notifyCommand()` to return that absolute path directly (drop the `bun ` prefix).
- [x] 2.3 Update `prepareNotifyScript()` to verify the resolved path exists and is executable;
      keep the typed `HookInstallFailed` error when it is absent (update the message — no longer
      "run: bun run build").

## 3. Remove the runtime-based client

- [x] 3.1 Delete `packages/platform-bun/src/notify.ts`.
- [x] 3.2 Remove the `build` script in `packages/platform-bun/package.json` that emits
      `notify.mjs` into `$ATHING_DIR` (no build step remains for the hook client).
- [x] 3.3 Update the adapter `HOOK_MARKER` in `hook-installer.ts` from `"notify.mjs"` to
      `"athing-notify"` so install/uninstall idempotency still detects the entry.

## 4. Tests

- [x] 4.1 `bun test` in platform-bun: start a throwaway local unix-socket receiver, exec
      `bin/athing-notify` with a payload on stdin and `ATHING_BRIDGE_URL`/`ATHING_SESSION_TOKEN`/
      `ATHING_SESSION_ID` set; assert the receiver got the exact body plus both auth headers.
- [x] 4.2 No-bridge case: unset `ATHING_BRIDGE_URL`, exec the client, expect exit 0 and no delivery.
- [x] 4.3 Fire-and-forget: point the bridge at a dead/slow endpoint, exec the client, expect exit 0
      within the bound and no thrown error.
- [x] 4.4 Update/extend `packages/platform-bun/tests/supervisor.test.ts` (or a new
      `ingress.test.ts`) to assert `notifyCommand()` resolves the bin path and `prepareNotifyScript`
      throws `HookInstallFailed` when absent.
- [x] 4.5 Run `bun test` across the workspace + the daemon hook-ingress tests to confirm the
      end-to-end hook path (agent -> client -> ingress -> relay) stays green.

## 5. Docs

- [x] 5.1 Note the runtime-free shell client and the `curl`-on-PATH assumption (macOS/Linux v1) in
      the platform-bun README / hook docs; do not introduce a fallback runtime.

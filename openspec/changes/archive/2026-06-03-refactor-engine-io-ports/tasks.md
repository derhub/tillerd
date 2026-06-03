## 1. Port contracts in `@athing/sdk`

- [x] 1.1 Add `DaemonTransport` interface to `@athing/sdk`: `send(message, body?)`,
      `subscribe(sessionId, handler): () => void`, `list(): Promise<string[]>`, `disconnect()`,
      `onClose(cb)`. Reference the existing wire frame/message types.
- [x] 1.2 Add `FileSource` interface to `@athing/sdk`: `size(path): Promise<number | null>`
      (null when absent), `read(path, offset, length): Promise<Uint8Array>`.
- [x] 1.3 Add `Logger` interface to `@athing/sdk`: `debug/info/warn/error` (the level surface the
      engine uses), decoupling the engine from the node-coupled `@athing/logger`.
- [x] 1.4 Resolve the open question on the minimal `send` overload set before finalizing 1.1
      (confirm `{ op: "unsubscribe" }` and binary-body handling stay in the interface vs Bun impl).

## 2. `TranscriptReader` reads through `FileSource` (async)

- [x] 2.1 Write failing tests: a fake `FileSource` drives `TranscriptReader`; assert content
      events and the transcript-unavailable error derive from the fake, with no `node:fs` access.
      (Added `packages/engine/tests/content.test.ts`.)
- [x] 2.2 Refactor `TranscriptReader` to take an injected `FileSource`; make `readDelta` async;
      replace `fs.statSync/openSync/readSync` with `FileSource.size`/`read`.
- [x] 2.3 Verify hook/exit call sites tolerate the now-async read (fire-and-forget, emit via
      existing handler sets); confirm content/status/exit ordering unchanged.

## 3. `AgentSessionProxy` consumes the transport interface

- [x] 3.1 Write failing test: a fake `DaemonTransport` injected into the proxy receives the spawn
      and control messages and feeds frames back; assert session events match.
- [x] 3.2 Change the proxy to depend on the `DaemonTransport` interface (not the concrete client)
      and to receive a `FileSource` to construct its `TranscriptReader`.
- [x] 3.3 Replace `randomBytes` (`node:crypto`) token generation with Web Crypto
      (`crypto.getRandomValues`); replace the `process.cwd()` default in `fillProxyOptions` by
      requiring `cwd` from `SessionOptions`.

## 3a. Ambient-primitive and logger removal in the engine

- [x] 3a.1 Write failing test: constructing the engine with an injected `Logger` routes all
      engine diagnostics through it; no `@athing/logger`/`createLogger` is referenced.
      (`content.test.ts` asserts parse diagnostics route through the injected logger; grep gate
      confirms `@athing/logger` is gone from the engine.)
- [x] 3a.2 Replace `randomUUID` (`node:crypto`) in `engine.ts` with `crypto.randomUUID()`.
- [x] 3a.3 Replace `createLogger()` with the injected `Logger` from `deps`; remove the
      `@athing/logger` import from `packages/engine`.

## 4. `createEngine(deps)` injection

- [x] 4.1 Exercise `createEngine({ transport, fileSource, logger, hooksSocketPath })` driving a
      full session. (`tests/integration/engine.test.ts` builds the deps and drives real PTY
      sessions end-to-end; proxy unit tests cover the same wiring with fakes.)
- [x] 4.2 Change `createEngine()` → `createEngine(deps)` carrying `transport`, `fileSource`,
      `logger`, and `hooksSocketPath`; thread them into proxies; remove lazy
      `getDaemonClient`/`adoptOrSpawn`, `checkCliVersion`, and `prepareNotifyScript` from the
      engine.
- [x] 4.3 Update `engine.shutdown` to unsubscribe sessions and `disconnect()` the transport
      without spawning/killing the daemon (D4).
- [x] 4.4 Remove `adoptOrSpawn` and `DaemonClient` from `packages/engine/src/index.ts` exports.

## 5. Relocate bootstrap + Bun impls to `apps/server`

- [x] 5.1 Move `daemon/supervisor.ts` (`adoptOrSpawn`, manifest, `HOOKS_SOCK`),
      `pty/resolve.ts` (`resolveBinary`, `checkCliVersion`), `ingress/install.ts`
      (`prepareNotifyScript`, `notifyCommand`), and the `ingress/notify.{ts,mjs}` callback script
      out of `packages/engine` into `apps/server`.
- [x] 5.2 Adapt `DaemonClient` to implement `DaemonTransport`; add `BunFileSource` wrapping
      `node:fs` over the `FileSource` interface, in `apps/server`.
- [x] 5.3 Audit all importers of the moved symbols (engine tests, dev-harness, e2e) and update
      call sites — resolve the open question on non-`apps/server` importers.

## 6. Wire `apps/server` bootstrap

- [x] 6.1 In `apps/server` startup: resolve the agent binary, verify CLI version, spawn/adopt the
      daemon to obtain a connected `DaemonTransport`, prepare the notify script, install hooks on
      the adapter, construct `BunFileSource`, and adapt the existing node logger to the `Logger`
      contract.
- [x] 6.2 Surface `VersionUnsupported` / `HookInstallFailed` / `BinaryNotFound` at startup
      (before accepting sessions). `apps/server` bootstrap calls `checkCliVersion` +
      `prepareNotifyScript` before `Bun.serve`; `apps/server/tests/bootstrap.test.ts` asserts all
      three typed errors fire from the host path.
- [x] 6.3 Call `createEngine({ transport, fileSource, hooksSocketPath })` and confirm the
      WebSocket session and terminal flows behave as before.

## 7. Verification

- [x] 7.1 `bun test` green across `@athing/sdk`, `@athing/engine`, `apps/server`.
- [x] 7.2 `turbo run check-types` clean (engine no longer references Bun/Node I/O types).
- [x] 7.3 Grep gate over `packages/engine/src`: no `node:` import (`fs`/`net`/`crypto`/`path`/
      `os`/`child_process`), no `Bun.` call, no `process.` reference, no `require(`, and no
      `@athing/logger` import.
- [x] 7.4 Run the e2e flow to confirm no observable behavior change on the web build.
      (Integration suite green against a live daemon: 13 pass — engine drives real PTY sessions
      through the injected `@athing/platform-bun` transport + `BunFileSource`.)

## Context

The engine performs its own platform I/O. `engine.ts` lazily calls `adoptOrSpawn()` to spawn
and connect the daemon, calls `checkCliVersion()` and `prepareNotifyScript()` +
`adapter.installHooks()` at first `start()`, and constructs `AgentSessionProxy` with a concrete
`DaemonClient` (`Bun.connect` unix socket) and the `HOOKS_SOCK` path. The proxy builds a
`TranscriptReader` (`session/content.ts`) that reads the transcript with synchronous
`fs.statSync/openSync/readSync` on every relevant hook (read-on-hook, ADR-0006).

That couples the engine to a Bun/Node runtime through three distinct surfaces: the daemon
socket, transcript file reads, and startup process/filesystem probing. Only the first two are
exercised during the agent loop; the third is one-time bootstrap. This design decouples all
three so the same engine runs on the Bun web host today and inside a web view later, with no
observable behavior change on the web build.

## Goals / Non-Goals

**Goals:**

- The engine depends on injected I/O contracts, not on `Bun.connect` or `node:fs`.
- The engine package imports zero Bun/Node I/O.
- Web behavior is unchanged; the refactor is behavior-preserving (Red stays green).
- Engine unit tests can inject fakes for the daemon link and transcript reads.

**Non-Goals:**

- Any web-view or desktop implementation of the ports (later change).
- Changing the daemon, its wire protocol, or the read-on-hook content semantics.
- Changing agent-session, status, content, or persistence behavior.
- Keeping back-compat shims — pre-v1, the entry-point shape may break.

## Decisions

### D1. Three port interfaces live in `@athing/sdk` (type-only)

Add `DaemonTransport`, `FileSource`, and `Logger` as interface declarations in `@athing/sdk`.
They are pure contracts (no impl, no I/O), consistent with the sdk charter; `DaemonTransport`
references the wire frame/message types that already live in (or are moving to) sdk.

- `DaemonTransport` mirrors what `AgentSessionProxy` and the engine consume today:
  `send(message, body?)`, `subscribe(sessionId, handler): () => void`, `list(): Promise<string[]>`,
  `disconnect()`, `onClose(cb)`. Connection establishment is the owner's responsibility (D4).
- `FileSource` covers the read-on-hook delta read: `size(path): Promise<number | null>`
  (null when absent) and `read(path, offset, length): Promise<Uint8Array>`.
- `Logger` is the minimal level surface the engine already uses (`debug/info/warn/error`),
  decoupling it from the node-coupled `@athing/logger` (which imports `node:fs` and reads
  `process`).

*Why sdk:* contracts belong there; both the engine (consumer) and the host (implementer) can
depend on the interface without depending on each other's package. *Alternative:* define the
ports inside `@athing/engine` (consumer-owns-port, classic hexagonal) — rejected here because
the project centralizes contracts in sdk and points deps inward; a desktop implementer would
otherwise depend on the engine just for the type.

### D2. `FileSource` is async; `TranscriptReader` becomes async

The port is asynchronous because a web-view file read crosses an IPC boundary. The Bun impl
wraps synchronous `fs` calls in resolved promises. `TranscriptReader.readDelta` becomes async;
it is already invoked fire-and-forget from hook/exit handlers and emits results via its existing
handler sets, so callers do not need to await it.

*Why:* a sync-only port cannot back a web-view file read. *Alternative:* keep the port sync and
require hosts to provide a synchronous bridge — rejected, no synchronous fs is available in a
web view.

### D3. `createEngine` takes injected dependencies; bootstrap moves to the host

Change `createEngine()` to `createEngine(deps)` where `deps` carries the connected
`DaemonTransport`, a `FileSource`, and the hook socket path. The engine constructs proxies with
the injected transport and passes the `FileSource` down to the `TranscriptReader`. The engine no
longer lazily spawns the daemon, probes versions, or prepares the notify script.

The three host-only bootstrap concerns relocate to `apps/server` and run before/around engine
construction (resolve-then-inject — they are one-time startup, not loop-time, so they need no
port interface):

- `daemon/supervisor.ts` (`adoptOrSpawn`, manifest, `HOOKS_SOCK`) → host spawns/adopts the
  daemon and produces a connected Bun `DaemonTransport`.
- `pty/resolve.ts` (`resolveBinary`, `checkCliVersion`) → host resolves the agent binary and
  verifies its version, supplying the resolved command.
- `ingress/install.ts` (`prepareNotifyScript`, `notifyCommand`) → host prepares the notify
  script and installs hooks on the adapter, supplying the hook command.

*Why:* these do process/filesystem work that is not agent-loop logic and cannot run in a
web view; doing them in the host keeps the engine pure. *Alternative:* wrap them as additional
runtime ports — rejected as over-abstraction, since none is called during the loop.

### D4. Connection lifecycle is owned by the host; the engine only uses the transport

The host establishes the connection (via the bootstrap that spawns/adopts the daemon) and hands
the engine a connected `DaemonTransport`. The engine uses `send/subscribe/list` and, on
`shutdown`, unsubscribes its sessions and calls `disconnect()`. Killing the daemon process
remains the host's responsibility (on web, `apps/server`'s existing SIGTERM path; on desktop,
the native shell later).

*Why:* spawning/owning an OS process is host concern (D3); the engine should not decide when the
daemon lives or dies. *Alternative:* engine keeps lazy `adoptOrSpawn` ownership — rejected, it
reintroduces the Bun coupling this change removes.

### D5. Logger is injected; engine uses no ambient host primitives

The engine receives a `Logger` via `deps` (the web host passes the existing `@athing/logger`;
a later desktop host passes a console/Channel logger). Independently, the engine drops the two
remaining ambient-runtime dependencies:

- session ids and per-session tokens come from runtime-neutral **Web Crypto**
  (`crypto.randomUUID()`, `crypto.getRandomValues()`), available in a Bun host and a web view,
  replacing `node:crypto` (`randomUUID` in `engine.ts`, `randomBytes` in `proxy.ts`);
- the working directory is **required from `SessionOptions`** rather than defaulting to
  `process.cwd()` in `fillProxyOptions`; the host (web server or native core) supplies it.

*Why:* an injected logger and Web Crypto let the same engine run in a web view; requiring `cwd`
removes the last `process` reference. *Alternatives:* a browser-safe build of `@athing/logger`
via package export conditions — rejected in favor of an explicit injected port consistent with
the other contracts; injecting an `IdSource` instead of using Web Crypto — unnecessary, Web
Crypto is standard in both runtimes.

### D6. Bun implementations live in `@athing/platform-bun`

`DaemonClient` (Bun socket) becomes the Bun `DaemonTransport` impl; a `BunFileSource` wraps
`node:fs`; the supervisor/resolve/ingress bootstrap and the notify script live alongside. The
engine's `index.ts` stops exporting `adoptOrSpawn` and `DaemonClient`.

Originally scoped to `apps/server` (option B). The integration test suite turned out to be a
second Bun consumer that also needs these impls to construct the engine — the documented trigger
for the promotion — so they were extracted into a dedicated `@athing/platform-bun` package
consumed by both `apps/server` and `@athing/integration-tests`.

*Why:* a second Bun consumer materialized; the shared package removes a cross-package reach into
an app's internals. *Alternative:* keep them in `apps/server` and have the tests reach into its
`src/` — rejected, couples a test package to an app's private layout.

## Risks / Trade-offs

- [Making `FileSource`/`TranscriptReader` async could reorder content emission relative to
  status/exit events] → Preserve current ordering by keeping the same call sites and awaiting
  internally where ordering matters; cover with existing content/ordering tests before and after.
- [Moving version-check and hook-install out of `engine.start` changes when failures surface
  (host bootstrap vs first session)] → Host runs them at startup and surfaces the same typed
  errors (`VersionUnsupported`, `HookInstallFailed`) before accepting sessions; assert via tests.
- [Relocating `supervisor`/`resolve`/`ingress` touches `apps/server` wiring and the engine's
  public exports] → Pre-v1 allows the break; update `apps/server` in the same change and keep
  web e2e green.
- [The injected-deps signature is a breaking API change for any engine caller] → Only
  `apps/server` constructs the engine today; update it in lockstep. No shims (pre-v1).

## Migration Plan

1. Add `DaemonTransport`, `FileSource`, and `Logger` interfaces to `@athing/sdk`.
2. Refactor `TranscriptReader` to take an injected `FileSource` and become async (D2).
3. Refactor `AgentSessionProxy` to accept the transport via the `DaemonTransport` interface, pass
   the `FileSource` to its `TranscriptReader`, generate its token via Web Crypto, and take `cwd`
   from options (no `process.cwd()` default).
4. Change `createEngine()` → `createEngine(deps)` carrying `transport`, `fileSource`, `logger`,
   and `hooksSocketPath`; generate session ids via Web Crypto; remove lazy `adoptOrSpawn`,
   version check, notify-script prep, and `createLogger()` from the engine (D3/D5); stop
   exporting `adoptOrSpawn`/`DaemonClient`.
5. Move `supervisor.ts`, `resolve.ts`, `ingress/install.ts`, and the `ingress/notify.{ts,mjs}`
   callback script to `apps/server`; add `BunFileSource` and adapt `DaemonClient` as the Bun
   `DaemonTransport`.
6. Wire `apps/server`: bootstrap (resolve, version-check, spawn daemon, install hooks), construct
   the ports (incl. the existing node logger as `Logger`), supply `cwd`, call
   `createEngine(deps)`. Keep all web/e2e tests green.

Rollback: revert is a single change; nothing ships to users mid-flight. The web host is the only
consumer and is updated atomically.

## Open Questions

- Exact `DaemonTransport.send` overload set: today `DaemonClient.send(message, body?)` carries an
  optional binary body and also handles `{ op: "unsubscribe" }`. Confirm the minimal method set
  the interface must expose versus what stays Bun-impl-internal.
- Should `list()` stay on `DaemonTransport`, or move to a separate small capability the host
  calls during reconnect reconciliation? (Leaning: keep on the transport — the engine already
  uses it for `reconnect`/`listSessions`.)
- Importer audit (resolved): repo-wide scan shows only `apps/server` imports the moving symbols
  (`adoptOrSpawn`/`DaemonClient`); engine tests/harness will need updating to inject fakes.
- `ingress/notify.{ts,mjs}` is the hook callback script that runs in the *agent* process
  (`process.stdin`, `fetch` to the bridge). It is not engine-library runtime, so it does not
  block engine-in-web-view; it moves with the ingress relocation and its packaging for desktop
  is handled by the later desktop change.

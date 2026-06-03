## Why

The engine is the agent-loop machinery, but it currently reaches the operating system
directly: it opens the daemon connection itself, reads transcript files itself, and spawns
and probes processes at startup. That hard-wires the engine to a Bun/Node runtime and blocks
it from running in any other host (notably an embedded web view, the prerequisite for a native
desktop build). Decoupling the engine from its I/O — behind a small set of injected contracts —
lets the same agent-loop run unchanged on the web host (Bun) and, later, inside a desktop web
view, without changing any observable behavior today.

## What Changes

- Introduce three injected **contracts** the engine depends on instead of touching the platform
  directly:
  - a **daemon transport** contract — connect, send a frame, receive frames, close;
  - a **file source** contract — read a transcript file (used by the read-on-hook content path);
  - a **logger** contract — replaces the engine's direct use of the node-coupled logger.
- Remove the engine's remaining ambient-host dependencies so it relies on no global runtime
  primitives:
  - generate session ids and tokens via runtime-neutral Web Crypto (works in a Bun host and a
    web view), replacing `node:crypto`;
  - require the working directory from session options rather than defaulting to `process.cwd()`.
- Refactor the engine so the agent-session lifecycle (`start`, `reconnect`) accepts these ports
  by injection, plus the resolved values it needs at startup, rather than constructing them.
- Relocate the three host-only bootstrap concerns out of the engine into the web host
  (`apps/server`), since they perform process/filesystem work that does not belong in the
  agent loop and cannot run in a non-Bun host:
  - daemon spawn/adopt/manifest supervision,
  - hook-ingress command/script preparation,
  - agent-binary resolution and CLI version probing.
- Provide the Bun implementations of the ports in a `@athing/platform-bun` package (consumed by
  `apps/server` and the integration tests); the engine no longer imports any Bun/Node I/O.
- **BREAKING** (pre-v1, internal): the engine entry points change shape to receive injected
  ports and bootstrap values; the daemon-connection and transcript-reading internals move
  behind the contracts. No deprecation shims (per pre-v1 policy). No user-facing behavior
  changes on the web build.

## Capabilities

### New Capabilities

- `engine-platform-ports`: the contract by which the engine obtains its runtime I/O — the
  daemon transport, the file source, and the logger — through injected ports rather than
  constructing them; the set of bootstrap values (connected transport, file source, logger,
  resolved binary path, hook command, working directory) the host supplies; and the engine's
  freedom from ambient host primitives (no `process`, no `node:crypto`, no node-coupled logger).

### Modified Capabilities

<!-- None. This is a behavior-preserving refactor: the agent-session, status, content,
     persistence, and daemon contracts keep their existing requirements. Only the engine's
     internal coupling to the platform changes, which is not a spec-level behavior change. -->

## Impact

- **`packages/engine`**: `daemon/client.ts` (Bun socket) and `session/content.ts` (fs reads)
  move behind the port contracts; `daemon/supervisor.ts`, `ingress/install.ts`, and
  `pty/resolve.ts` relocate to the host; `engine.ts`/`proxy.ts` drop `node:crypto` (Web Crypto),
  `process.cwd()` (cwd required from options), and `createLogger()` (injected logger). The
  `ingress/notify.{ts,mjs}` callback script moves with the ingress relocation. Engine becomes
  free of Bun/Node I/O imports and ambient host primitives.
- **`@athing/sdk`**: gains the three port interface declarations — daemon transport, file
  source, logger (type-only contracts; zero impl, consistent with its charter).
- **`@athing/platform-bun`** (new package): home of the Bun port implementations and bootstrap
  functions (daemon transport, file source, supervise daemon, resolve binary, prepare hook
  command, notify script). **`apps/server`**: consumes it and wires the ports into the engine at
  session start. Web behavior unchanged.
- **Tests**: engine unit tests can now inject fake ports (no real socket or filesystem),
  improving isolation; existing web/e2e behavior must stay green.
- **Unblocks**: a later desktop change that supplies web-view implementations of the same
  ports without touching the engine.

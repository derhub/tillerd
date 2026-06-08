## 1. Logger contract (`@athing/logger`)

- [x] 1.1 Add `LogContext` / `AttrValue` and `Resource` types to the logger package.
- [x] 1.2 Add `child(context: LogContext): Logger` to the `Logger` interface.
- [x] 1.3 Change `createLogger(sessionId?)` to `createLogger(resource: Resource)`; stamp
      resource fields onto every record via pino base bindings.
- [x] 1.4 Implement `child` by wrapping `pino.child(context)`, recursing the wrapper so
      children compose and inner bindings win on collision.
- [x] 1.5 Keep `noopLogger`; add a no-op `child` returning itself.
- [x] 1.6 Update logger unit tests: resource on every record, context inheritance, child
      composition, collision precedence, one-JSON-record-per-line.

## 2. Shared attribute vocabulary (`@athing/sdk`)

- [x] 2.1 Add an `ATTR` constants object (`SESSION_ID="session.id"`, `PTY_PID="pty.pid"`,
      `HOOK_EVENT="hook.event"`, `COMPONENT="component"`, `FRAME_SEQ="frame.seq"`) plus
      resource-key constants; `as const`, string-literal only (no I/O, no logging import —
      preserves Web-API-only / runtime-neutral-core).

## 3. TypeScript consumers

- [x] 3.1 `@athing/engine` (`daemon/proxy.ts`, `engine.ts`): bind a per-session child once
      (`logger.child({ [ATTR.SESSION_ID]: id, [ATTR.COMPONENT]: "engine" })`); remove
      hand-threaded `sessionId` from `proxy.ts:188` and peers. Imports only the injected
      `Logger` interface — no logging library. (Implements spec requirement 7.)
- [x] 3.2 `apps/server`: construct with `Resource` (`service.name: "athing-server"`); bind
      `component` + `session.id` via `.child()` at the right scopes; route the ~14 operational
      `console.log`/`console.error` sites (terminal frames, daemon lifecycle, uncaught
      exception at `index.ts:108`) through the structured logger. Leave a deliberate
      human-facing startup line (`Server on http://...`) as-is if intended for the operator.
- [x] 3.3 `apps/cli`: construct with `service.name: "athing-cli"`; route the cli's own
      diagnostics through the logger. DO NOT touch `index.ts:23-24` (`out`/`err` piping the
      agent's PTY bytes to the user terminal) — that is passthrough output, not logging.
- [x] 3.4 Use `ATTR.*` constants at all TypeScript call sites (`session.id`, `pty.pid`,
      `hook.event`, `component`, `frame.seq`).
- [x] 3.5 Update affected TS tests (`engine` proxy test, server api test if logger-coupled).

## 4. Native daemon (`packages/daemon-pty`) — the only live daemon (TS `packages/daemon` is legacy, not instrumented)

- [x] 4.1 Add `tracing` and `tracing-subscriber` (JSON feature) to `Cargo.toml`.
- [x] 4.2 In `main.rs`, init a JSON subscriber writing to
      `ATHING_DIR/logs/daemon-<date>.log` (separate file from the TS `<date>.log`, with the
      daemon's own rotation), `EnvFilter` from `LOG_LEVEL`, current-span fields enabled, and
      resource fields (`service.name: "athing-daemon"`, `service.version`, `process.pid`).
- [x] 4.3 Introduce a per-session span (`info_span!("session", session.id = %id, ...)`) at the
      session boundary so events inherit `session.id`. Hand-mirror the `ATTR` key literals.
- [x] 4.4 Replace every `eprintln!` in `main.rs`, `server.rs`, `hook_ingress.rs` with a leveled
      `tracing` event carrying structured fields (no string-formatted context).
- [x] 4.5 Secrets guard: any raw PTY byte-dump events (the Rust equivalent of `pty.out`/`pty.in`
      hex dumps) MUST be emitted only at `debug`/`trace` and never at a default-on level, per
      ADR-0007's redaction clause. Confirm the same holds for the existing TS `pty.out`/`pty.in`
      debug logs — keep them at `debug`, do not promote.

## 5. Docs

- [x] 5.1 Document the context-binding + resource model and the per-language expression
      (TS child logger vs Rust span) where contributors will find it.

## 6. Verification

- [x] 6.1 `bun test` green across logger, sdk, engine, server, cli.
- [x] 6.2 `cargo test` / build green for `daemon-pty`; manual run shows JSON records with
      resource + `session.id` in `daemon-<date>.log`.
- [x] 6.3 No operational `console.*` remains in `apps/server` / `apps/cli` (excluding the
      intentional human-facing startup line and the cli PTY passthrough).
- [x] 6.4 Sample a record from each runtime; confirm it maps to the OTel log model fields
      (timestamp, severity, body, attributes, resource) and is one JSON object per line.
- [x] 6.5 Confirm `@athing/sdk` and `@athing/engine` import no logging library
      (interface-only), preserving the runtime-neutral-core constraint.

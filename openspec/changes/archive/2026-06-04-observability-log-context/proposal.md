## Why

ADR-0007 mandates "session-correlated structured logs," but today only the TypeScript side
has structured logging, and even there context is hand-threaded at every call site — miss a
field once and the record is an orphan that cannot be correlated. The native PTY daemon, which
owns the master fds and is where processes leak or hang, logs through raw `eprintln!`: no level,
no structure, no context. Logs cannot be reliably grouped by session, component, or process,
and nothing is ready for a downstream collector to ingest. The missing capability is ambient
**log context binding** plus a per-process **resource** identity, expressed natively in each
language and shaped so an OpenTelemetry collector can map records without transforms.

## What Changes

- Add a context-binding contract to the logger: a child logger that binds structured context
  once, which every record in scope inherits — replacing per-call-site field threading.
- Introduce a per-process **resource** identity (service name, version, instance, host, pid)
  stamped onto every record at construction.
- **BREAKING**: `createLogger(sessionId?: string)` becomes `createLogger(resource)`; the
  `Logger` interface gains `child(context)`. The single optional `sessionId` parameter is
  removed in favor of `.child({ "session.id": id })`. (Pre-v1; no back-compat shim.)
- Bring the native daemon up to the structured-logging bar by adopting the `tracing` crate:
  spans carry context, events inherit it, output is JSON-to-file. Replaces all `eprintln!`.
- Standardize context attribute keys across both runtimes (e.g. `session.id`, `pty.pid`,
  `hook.event`, `component`) using dotted, collector-friendly names.
- Emit structured JSON, one record per line, to a consistent log location in both runtimes,
  with field semantics (timestamp, severity, body, attributes, resource) that map cleanly to
  the OpenTelemetry log data model. No exporter, collector, metrics, or tracing pipeline is
  built here — OTel readiness means the shape is correct, not that a pipeline is wired.

## Capabilities

### New Capabilities

- `observability-logging`: structured, context-bound logging across the TypeScript and Rust
  runtimes — the logger context-binding contract (child loggers), per-process resource
  identity, standardized attribute keys, and OTel-collector-friendly JSON-to-file output.

### Modified Capabilities

<!-- None: no existing spec defines logging requirements. ADR-0007 states the obligation but
     no capability spec currently owns it; this change creates that spec. -->

## Impact

- `@athing/logger`: `Logger` interface (`child`), `createLogger` signature, resource type.
  **BREAKING** for all consumers below.
- Consumers updated to bind context via `.child()` and construct with a resource:
  `apps/server`, `apps/cli`, and `@athing/engine` (`daemon/proxy.ts` — currently hand-threads
  `sessionId`), plus their tests. Operational `console.*` sites in `apps/server` are routed
  through the structured logger; the cli's PTY-passthrough output is left untouched.
- `@athing/sdk`: gains an `ATTR` constants object (string literals only — no I/O, no logging
  import; Web-API-only / runtime-neutral-core preserved).
- `@athing/engine`: uses only the injected `Logger` interface (no logging library imported);
  the `.child` addition is to a type it already consumes by DI.
- `packages/daemon-rs`: add `tracing` + `tracing-subscriber` (JSON layer to its own
  `daemon-<date>.log`); replace every `eprintln!`; init resource + subscriber in `main.rs`.
  New Cargo dependencies.
- `packages/daemon` (TS): **legacy, not instrumented** — `daemon-rs` is the live daemon. Only
  the exported `HOOKS_SOCK` constant remains in use by `apps/server`.
- Honors ADR-0007 (session-correlated structured logs); sets up a future ADR for the
  context-binding + resource pattern and the per-language expression (TS child logger vs Rust
  `tracing` spans).

## Context

ADR-0007 obliges every engine to emit "session-correlated structured logs." That obligation is
only partially met. The TypeScript logger (`@athing/logger`, pino-based) produces structured
JSON and can bind a single `sessionId`, but consumers re-pass correlation fields at each call
site by hand — when a field is omitted (e.g. `pty-transport` logging `spawning pty` with no
session), the record becomes an orphan that cannot be grouped. The native daemon
(`packages/daemon-pty`), which per ADR-0008..0011 owns the PTY master fds and survives upgrades,
logs exclusively through `eprintln!`: unstructured text to stderr, no level, no context.

The gap is not "logging to a file" — it is **ambient context binding** (bind correlation once,
inherit it on every record in scope) and a **per-process resource identity** (which component,
version, host, pid emitted the record). These two concepts are exactly the OpenTelemetry log
data model's `Attributes` and `Resource`. Shaping our output around them now makes the logs
collector-ingestible later without building any pipeline.

In-force ADRs that constrain this design:

- ADR-0003 (ports-and-adapters with composition-root DI): the core depends inward on
  interfaces; concrete wiring happens at the composition root.
- ADR-0007 (reliability/operability contract): session-correlated structured logs; opt-in,
  redacted raw-I/O capture (the raw-capture clause is acknowledged but out of scope here).

No ADR is superseded by this change.

## Goals / Non-Goals

**Goals:**

- Add a context-binding contract (`child`) to the `Logger` interface so correlation context is
  bound once and inherited, eliminating per-call-site field threading.
- Introduce a per-process resource identity stamped on every record.
- Bring `daemon-pty` to the structured-logging bar by adopting `tracing` (JSON to file).
- Standardize a small set of dotted attribute keys across both runtimes.
- Keep the output shape mappable to the OpenTelemetry log data model.

**Non-Goals:**

- No OTLP exporter, collector, Grafana/Tempo/Prometheus, or any running pipeline.
- No metrics and no distributed tracing spans as product features (the Rust `tracing` crate is
  used as a structured-logging API, not to ship traces).
- No cross-process trace-context propagation; correlation is by `session.id` attribute, not a
  propagated `traceparent`. The ADR-0009 wire format is unchanged.
- No raw-I/O capture (ADR-0007's redacted raw capture is a separate future change).
- No `AsyncLocalStorage`-based implicit propagation in TypeScript.

## Decisions

### D1: Context binding via explicit child loggers (TS), spans (Rust)

The missing capability is ambient context. Each runtime already has a native expression of it,
so we use each language's grain rather than forcing one shape across both.

- **TypeScript**: extend the `Logger` interface with `child(context): Logger`. pino already
  implements this (`pino.child(bindings)`); the wrapper currently calls it internally but does
  not expose it. We surface it. Children compose (child-of-child merges context).
- **Rust**: adopt `tracing`. A span (`info_span!(...)` + guard, or `#[instrument]`) carries
  fields; events inside inherit them. The span _is_ the child logger.

Alternative considered — a hand-rolled `Logger` trait in Rust mirroring the TS interface
line-for-line. Rejected: it reimplements what `tracing` already provides, forgoes the
`tracing-opentelemetry` bridge, and fights the ecosystem grain. Symmetry of _concept_ matters;
symmetry of _signature_ across languages does not.

Alternative considered — `AsyncLocalStorage` for implicit context in TS. Rejected for now: the
codebase already threads `logger` through DI/`opts`, so explicit `.child()` fits the existing
flow; implicit propagation adds runtime machinery for no current need.

### D2: Resource identity at construction

`createLogger` takes a `Resource` (service name, version, optional instance id, host, pid)
stamped onto every record. This replaces the single optional `sessionId` parameter — session is
now ordinary bound context (`.child({ "session.id": id })`), not a privileged constructor arg.
In Rust the resource is set once on the `tracing` subscriber at `main.rs` init.

Rationale: "which component/version/host emitted this" is process-static identity — the OTel
`Resource` concept — and belongs at construction, not on each call. Keeping `sessionId` as a
special case would perpetuate the privileged-single-field design that caused orphan records.

### D3: Attribute key vocabulary (dotted, collector-friendly)

A small shared set of dotted keys, used identically in both runtimes:
`service.name`, `service.version`, `service.instance.id`, `host.name`, `process.pid`
(resource); `session.id`, `pty.pid`, `hook.event`, `component`, `frame.seq` (attributes).
Dotted names map directly to OTel semantic-convention attributes; a collector groups on them
with no transform.

### D4: OTel-readiness is shape, not pipeline

We emit one JSON record per line with fields that map to the OTel log model
(timestamp, severity, body, attributes, resource). Field _names_ are deliberately not bikeshed:
any collector renames on ingest. What matters is that the structured context _exists_ and is
consistent. The `tracing-opentelemetry` / pino-OTel bridges remain a future wiring step behind
the same plug points, requiring no change to call sites.

### D5: Core stays Web-API-only (ADR-0003, runtime-neutral-core)

`@athing/sdk` and `@athing/engine` continue to see only the injected `Logger` interface; they
never import pino, `tracing`, or any logging library. The interface change (`child`) is
additive to a type they already consume by DI, so the inward-pointing dependency rule holds.

## Risks / Trade-offs

- [BREAKING signature change `createLogger(sessionId?)` → `createLogger(resource)` ripples to
  every consumer] → Pre-v1, no shim allowed by project policy; change is mechanical
  (server, cli, TS daemon, tests) and caught at compile time by the type system.
- [Per-language asymmetry (TS child object vs Rust span) could confuse contributors] → Document
  the shared concept (bind context → inherited attributes → OTel mapping) in the spec and ADR;
  the asymmetry is idiomatic, not accidental.
- [New Rust dependencies `tracing` + `tracing-subscriber` enlarge the daemon build] → Both are
  the de-facto ecosystem standard, lightweight, and unlock the future OTLP bridge.
- [Hex-dump `pty.out`/`pty.in` debug logs may contain secrets] → Out of scope here, but flag:
  those remain `debug`-level and are governed by ADR-0007's redaction clause; this change must
  not promote them to a default level.
- [Explicit child threading can still be forgotten, re-orphaning a record] → Mitigated by
  binding session context at the highest scope (per-session logger) so leaf call sites inherit
  it without action; lint/review catches new top-level `createLogger` misuse.

## Migration Plan

1. Land the `@athing/logger` change: add `child`, switch `createLogger` to take a `Resource`,
   keep `noopLogger`.
2. Update live TS consumers (`apps/server`, `apps/cli`, `@athing/engine`) to construct with a
   resource and bind `session.id`/`component` via `.child()` at the right scope; route
   operational `console.*` through the logger. The TS `packages/daemon` is legacy (superseded by
   `daemon-pty`) and is not instrumented. Compiler flags every call site.
3. Add `tracing` + `tracing-subscriber` to `daemon-pty`; init subscriber + resource in
   `main.rs` writing to its own `daemon-<date>.log`; replace every `eprintln!` with a leveled
   `tracing` event under a session span.
4. No runtime rollout risk: logging is internal. Rollback = revert; no data migration, no wire
   change, no persisted format depends on this.

## Resolved Decisions

- **Attribute-key vocabulary**: exported as constants from `@athing/sdk` (e.g.
  `ATTR.SESSION_ID = "session.id"`). These are pure string-literal data — no I/O, no logging
  library — so the sdk's Web-API-only / runtime-neutral constraint holds. TypeScript consumers
  import them, gaining compile-time typo protection. `daemon-pty` hand-mirrors the same literals
  in Rust (no cross-language import exists); cross-runtime consistency is enforced by review and
  the shared vocabulary requirement in the spec, not by a shared type.
- **`daemon-pty` log destination**: its own file, `ATHING_DIR/logs/daemon-<date>.log`, distinct
  from the TypeScript `ATHING_DIR/logs/<date>.log`. Avoids two processes contending on one
  file's append path and gives the native daemon independent rotation. Cross-runtime correlation
  happens downstream by `service.name` + `session.id`, which the collector groups on.

## Open Questions

- No in-force ADR needs revisiting; this design is additive to ADR-0003 and advances ADR-0007.

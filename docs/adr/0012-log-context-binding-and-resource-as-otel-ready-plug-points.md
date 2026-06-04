# 0012. Log context binding and resource as OTel-ready plug points

- Status: accepted
- Date: 2026-06-04

## Context

ADR-0007 commits every engine to "session-correlated structured logs," but the commitment was
met unevenly. The TypeScript logger emits structured JSON yet correlation context is re-supplied
by hand at each call site, so an omitted field yields an uncorrelatable orphan record. The
native PTY daemon (ADR-0008..0011), which owns the master fds and is the component most likely
to leak or hang, logged only through unstructured `eprintln!` — no level, no structure, no
context. The durable question this raises is not "where do logs go" but "how is correlation
context modeled," and how that model stays vendor-neutral so an OpenTelemetry collector can
ingest the output later without forcing a logging pipeline into the core now.

Two cross-cutting concepts answer it: binding correlation context once so it is inherited by
every record in scope (the OpenTelemetry `Attributes` concept), and a per-process identity
stamped on every record (the OpenTelemetry `Resource` concept). These must hold across two
runtimes (TypeScript and Rust) while respecting ADR-0003 (ports-and-adapters: the core depends
inward on interfaces, concrete wiring lives at the composition root).

## Decision

Observability is instrumented against a vendor-neutral logging contract in each runtime, with
the OpenTelemetry SDK left as a future wiring choice at the composition root — never a core
dependency. Concretely:

- The logging contract SHALL express **context binding**: a child logger binds structured
  context once and every record in scope inherits it, with children composing. Correlation is
  ambient, not re-passed per call.
- A logger SHALL be constructed with a **resource** identity (service name/version, optional
  instance, host, pid). Session correlation is ordinary bound context, not a privileged
  constructor parameter.
- The contract is expressed in each language's native grain, not by a shared signature:
  TypeScript via the `Logger` interface's `child()` (over pino), Rust via `tracing` spans and
  fields. Symmetry of concept is required; symmetry of signature is not.
- Output is structured JSON whose field semantics map to the OpenTelemetry log data model
  (timestamp, severity, body, attributes, resource). OTel-readiness means the shape is correct,
  not that any exporter, collector, or pipeline is wired.
- The core (`@athing/sdk`, `@athing/engine`) sees only the injected `Logger` interface and
  imports no logging library, preserving the ADR-0003 inward-pointing dependency rule and the
  runtime-neutral-core constraint.

This ADR is additive: it advances ADR-0007's structured-logging clause and is coherent with
ADR-0003. It supersedes no prior ADR.

## Consequences

- Records are reliably groupable by session, component, version, host, and pid; orphan records
  from a forgotten correlation field are designed out by binding context at the highest scope.
- Adopting OpenTelemetry later (pino-OTel bridge, `tracing-opentelemetry`) is a wiring change
  behind the same plug points, with no edits to call sites or to the core.
- The two-runtime asymmetry (explicit child object in TS, implicit span context in Rust) is a
  permanent, intentional shape contributors must learn; it is idiomatic to each language.
- A breaking constructor change (`createLogger(sessionId?)` → `createLogger(resource)`) ripples
  to all consumers, accepted pre-v1 with no compatibility shim.
- This ADR deliberately scopes out metrics, distributed tracing spans as a product feature,
  cross-process trace-context propagation, and ADR-0007's redacted raw-I/O capture; each remains
  a separate future decision behind the same plug-point discipline.

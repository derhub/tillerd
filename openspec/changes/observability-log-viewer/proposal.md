## Why

0.0.6 made the daemon emit structured, OpenTelemetry-shaped JSON log records, but there
is no way to read them inside the app, and the other services do not yet log uniformly.
An operator must open rolling log files or read raw stderr by hand to see what the system
is doing. A single in-app view over all services' structured logs is the first
observability surface (0.0.7) and the foundation that health surfacing (0.0.8) builds on.

## What Changes

- Every long-lived service emits its structured JSON log records to a rolling, per-service
  file under the runtime logs directory (`TILLERD_DIR/logs/<service>.<date>.log`),
  generalizing the pattern the daemon already implements. This unifies the format and
  location so one viewer can read them all. **The MCP gateway moves off plain stderr.**
- The renderer reads logs through a host-agnostic source port (`list` / `size` / `read`); the
  desktop adapter ships now (a Tauri log-directory listing plus the existing file-read), and a
  server/web adapter lands additively before v1 without changing the viewer.
- The renderer gains a global log-viewer **route** (app-shell navigation, not a session
  surface). It tails the rolling JSON files through the port (poll size, read the delta),
  parses each line, merges records across files by timestamp, and offers a level filter,
  free-text search, and `component` / `session.id` facets.
- Records keep their 0.0.6 field semantics end to end (timestamp, severity, body,
  attributes, resource); the viewer renders those fields without reshaping them.

**Non-goals (this change):** a real-time push stream (file polling gives near-live; a true
stream is a later change if latency proves insufficient), durable retention beyond the
rolling files' own daily rotation, export to an external collector / OTLP, redaction beyond
what 0.0.6 already guarantees, and building the server/web log-source adapter. The viewer
reads through a host-agnostic source port; only the desktop (Tauri) adapter ships in 0.0.7,
with the server adapter expected to land additively before v1.

## Capabilities

### New Capabilities

- `observability-log-viewer`: the host lists the structured log files, and the renderer
  global route tails them through the file-read transport, merges records across files by
  time, and filters/searches by level, free text, and `component` / `session.id`.

### Modified Capabilities

- `observability-logging`: every long-lived service SHALL write its structured JSON records
  to a rolling, per-service file under the runtime logs directory, not to stderr or an
  ad-hoc target. Today only the daemon does; this generalizes it to a uniform sink so the
  viewer has one source of truth.

## Impact

- **Services (Rust):** a shared tracing-init helper that writes JSON to
  `logs/<service>.<date>.log` (`tracing_appender::rolling`), replacing each binary's ad-hoc
  subscriber setup. The daemon's existing `init_tracing` is the template; the gateway,
  orchestrator host, and gate adopt it.
- **Desktop host (Rust / Tauri):** a command that lists the log files in the runtime logs
  directory (the `list` operation of the source port; `size` / `read` already exist).
- **Renderer (TS / React SPA):** a new top-level `logs` route, a navigation entry to reach
  it, JSON-line parsing + cross-file time-merge, a bounded tail window with load-older, and
  the filter/search UI, and the host-agnostic source port with its desktop (Tauri) adapter over
the existing `TauriFileSource` — built on the 0.0.6 design tokens.
- **Builds on:** `observability-logging` (record shape and the daemon's file sink),
  `runtime-paths` (the logs directory), the renderer file-read transport
  (`apps/ui/app/lib/transport/file-source.ts`), and file-based routing
  (`apps/ui/app/routes.ts`).
- **Deliberately not touched:** the 0.0.6 surface model, the service contract, and the
  daemon wire protocol. The earlier candidate of an orchestrator log-stream over the
  service contract is dropped in favor of tailing the files that already exist.
- **Risk:** near-live latency is bounded by the poll interval (acceptable for logs); the
  uniform-sink change touches every service's startup path (mechanical, covered per service
  by tests).

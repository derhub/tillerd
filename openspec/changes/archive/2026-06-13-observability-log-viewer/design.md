## Context

0.0.6 logging is uneven. The daemon writes structured JSON to a rolling daily file at
`TILLERD_DIR/logs/daemon.<date>.log` (`apps/daemon-pty/src/main.rs:27-61`, via
`tracing_appender::rolling`). The MCP gateway writes plain text to stderr
(`apps/mcp-gateway/src/bin/gateway.rs:7-9`). The orchestrator host and gate have no JSON
file sink. Each binary configures its own `tracing` subscriber; there is no shared helper.

Services run as detached daemons: the launcher spawns a backend, waits for control-socket
reachability, and the child writes its own manifest (`crates/process-launch/src/probes.rs`,
`spawn.rs`). The orchestrator never holds a child's stdout handle, so in-process stdout
capture is not available, and an adopted service was started by a different process entirely.

The renderer is a React Router framework-mode app with file-based routes
(`apps/ui/app/routes.ts`). It already reads host files off the hot path through
`TauriFileSource` — `size(path)` and `read(path, offset, length)` over Tauri `invoke`
(`apps/ui/app/lib/transport/file-source.ts:9-20`) — and already subscribes to host status
through the orchestrator client's `listen` pattern (`apps/ui/app/lib/transport/orchestrator.ts`,
`useDesktopHost.tsx:38`). The log files live on the desktop host where the services run. A
web/server host is expected before v1 (`apps/server`), so the viewer's source must be
host-agnostic rather than Tauri-bound, even though only the desktop adapter ships in 0.0.7.

## Goals / Non-Goals

**Goals:**

- One in-app view over the structured logs of every service.
- Uniform structured JSON sink: every service writes to `logs/<service>.<date>.log`.
- Near-live tail plus on-demand history, reusing the existing file-read transport.
- Filter by level and free text; facet by `component` and `session.id`.
- The viewer reads through a host-agnostic log-source port; the desktop (Tauri) adapter ships
  now, and a server/web adapter lands additively before v1 without changing the viewer.
- No change to the surface model, the service contract, or the wire protocol.

**Non-Goals:**

- A real-time push stream. Polling gives near-live; a stream is a later change if latency
  proves insufficient.
- Retention beyond the rolling files' own daily rotation; external collector / OTLP export.
- Redaction beyond what 0.0.6 emits (records are already redacted at source via the
  `redact` crate).
- Building the server/web log-source adapter. Only the desktop adapter ships in 0.0.7; the
  port is designed so the server adapter is additive (see Decisions), expected before v1.

## Decisions

### Source: tail the rolling JSON files

Tail the files the daemon already writes and that every other service will write, rather
than build an orchestrator-aggregated stream. The records already persist as JSON files,
and the renderer already has a file-read transport, so this is the smallest change that
delivers all logs and touches no frozen contract.

Alternatives: child-stdout capture (impossible under the detached-daemon model);
orchestrator log-stream over the service contract (real-time and adopted-safe, but the
largest scope, and it duplicates the daemon's existing file logging). The stream stays open
as a follow-up if poll latency is insufficient.

### Uniform sink: one shared tracing-init helper

Extract the daemon's `init_tracing` into a shared helper, `init_file_tracing(service, dir)`,
that builds a daily `tracing_appender::rolling` JSON layer writing to
`logs/<service>.<date>.log`, honoring the existing `LOG_LEVEL` / `EnvFilter` behavior, and
stamps the `service.name` / `service.version` resource. Every binary calls it: the daemon
keeps its current behavior, the gateway moves off stderr, the orchestrator host and gate gain
a file sink.

Alternative: leave per-binary setups as-is (rejected — non-uniform format and location make
"all logs" impossible).

### Helper home: a module in `tillerd-paths`

The helper is a module in the existing `tillerd-paths` crate (`init_file_tracing`), not a new
crate. `tillerd-paths` is the one crate all five log emitters already depend on (daemon, gate,
gateway, desktop host, and `service-host`), and it owns the runtime layout the logs dir lives
under. `service-host` was rejected because the desktop host does not depend on it
(`apps/desktop/src-tauri/Cargo.toml`) and would otherwise pull in the hosting machinery solely
to log. A standalone `tillerd-observability` crate was rejected per the project's no-new-crate
preference; the cost is that `tillerd-paths` gains `tracing-subscriber` + `tracing-appender`
dependencies (it otherwise only depends on `dirs`).

### Defaults

- Poll interval: 1s. `size(path)` is a cheap stat; logs are not a hot path.
- Live window: the most recent 2000 merged records held in memory.
- Initial backfill: the trailing 256 KB of each file on open.
- Load-older: read a further 64 KB per request by lowering the offset.

These are starting values, tunable later; they are not a contract.

### Host-agnostic log-source port

Define one renderer-side port the viewer depends on, with three operations:

- `list()` returns the available log files (name + size).
- `size(file)` returns current byte length (null when absent).
- `read(file, offset, length)` returns the raw byte range, short at EOF.

`size`/`read` mirror the existing `TauriFileSource`; `list` is the one new operation (the
renderer cannot enumerate a directory through `TauriFileSource`, which reads a known path).

The desktop adapter ships in 0.0.7: a Tauri command that lists the runtime logs directory,
plus the existing `file_size` / `file_read`. The server/web adapter is deferred but the port
is shaped for it - `list` maps to an index endpoint, `size` to a `HEAD` / content-length, and
`read` to an HTTP `Range` GET, or all three over the existing WebSocket transport. The tail,
merge, parse, and filter logic sits above the port and is identical across hosts, so adding
the server adapter before v1 is additive and touches no viewer logic.

### Near-live tail and merge in the renderer

Per file, poll `size(path)`; when it grows, `read` the new byte range and split on newlines
into JSON records. Hold a bounded window of the most recent records across all files, merged
by parsed timestamp. "Load older" reads further back by lowering the offset. A partial
trailing line (write mid-flush) is buffered until its newline arrives. Poll interval is
modest (logs are not a hot path, per the file-source contract).

The host primitive already exists and is tested: `apps/desktop/src-tauri/src/files.rs` exposes
`file_size(path) -> Option<u64>` and `file_read(path, offset, length)` (`read_bytes` seeks to
the offset and returns a short read at EOF), with offset and EOF coverage. Follow is poll +
delta read over these two commands; no new host code is needed for the byte path.

This is why the uutils coreutils `tail` (`uu_tail`) is not adopted. Its only public entry is
`uumain(args)`; the follow loop is the private `mod follow`, `Settings`-driven and writing to
stdout, so it is not callable as a library - reuse means running it as a subprocess and
scraping stdout, the pattern this change rejects, and a host-side `tail -f` is the push model
the change defers. It would also pull `clap`, `uucore`, and `fluent` (i18n) for what is
host-side a `seek` + read already implemented and tested in
`apps/desktop/src-tauri/src/files.rs`. No file watcher (`notify`/`inotify`/`kqueue`) is pulled
in for the same reason. File rotation needs no inode tracking: `tracing_appender` rotates by
daily filename, so a new day is a new file the directory listing surfaces and the renderer
tails from zero while the prior file stops growing (uu_tail's own polling rotation-follow is
marked incomplete in-source).

### Record model and filtering

Each line maps to a view record: `{ timestamp, level, body, attributes, resource }` with
`component`, `session.id`, and `correlation_id` surfaced from attributes. Filtering and
search run client-side over the in-memory window: level threshold, free text over body and
attributes, and facet selection on `component` / `session.id`.

### Route and navigation

Add a global route under the shell layout in `apps/ui/app/routes.ts`
(`route("logs", "routes/_shell.logs.tsx")`) and a chrome navigation entry to reach it. It is
app-shell scoped, not a session surface, so it does not enter the placement model.

## Risks / Trade-offs

- Poll latency, not instant → modest interval; acceptable for logs; stream deferred as a
  follow-up.
- Uniform sink touches every service's startup path → one shared helper, covered per service
  by a test asserting the file sink; modifies the `observability-logging` spec (flagged,
  additive and uniform).
- Unbounded file growth → daily rotation already caps each file; the renderer holds a bounded
  tail window and reads older ranges on demand.
- Cross-file timestamp merge can interleave imperfectly under clock skew → merge by parsed
  timestamp with a small reorder tolerance; records are single-process ordered within a file.
- Web host has no file access → desktop-only v1, documented as a non-goal.

## Migration Plan

Additive; no data migration. Ship the shared init helper and per-service adoption, the host
listing command, and the renderer route together. Rollback removes the route and reverts the
per-service sink to its prior target.

## Open Questions

None outstanding. The three prior unknowns are settled above: the helper home (a dedicated
`tillerd-observability` crate), web-host viewing (out of scope v1; desktop-only, see
Non-Goals), and the poll/window defaults (see Defaults).

## 1. Uniform structured file sink (observability-logging)

- [x] 1.1 Add `init_file_tracing(service, dir)` as a module in the existing `tillerd-paths`
  crate: a daily rolling JSON appender to `<dir>/logs/<service>.<date>.log`, the
  `service.name` / `service.version` resource, and the existing `LOG_LEVEL` / `EnvFilter`
  behavior; plus a `logs_dir_in(dir)` helper. Add `tracing-subscriber` + `tracing-appender`
  deps to `tillerd-paths`. Tests first, per the new `observability-logging` scenarios (one JSON
  line to the dated file; resource present).
- [x] 1.2 Refactor `apps/daemon-pty` to use `init_file_tracing`, preserving current behavior.
- [x] 1.3 Move `apps/mcp-gateway` off plain stderr to `init_file_tracing` (structured file logs).
- [x] 1.4 Wire `init_file_tracing` into the gate and the orchestrator host (desktop) startup.

## 2. Host log-file listing

- [x] 2.1 Add a desktop host command that lists `<runtime>/logs` and returns each file's name
  and size (via `tillerd-paths`). Test: present files listed with sizes; empty when none.

## 3. Renderer log-source port + desktop adapter

- [x] 3.1 Define the host-agnostic log-source port (`list` / `size` / `read`) in the renderer
  transport layer and implement the desktop adapter over the new list command plus the existing
  `TauriFileSource` size/read. Tests: adapter lists files with sizes; read is short at EOF.

## 4. Renderer tail and merge engine

- [x] 4.1 Implement the tail engine over the port: poll size, read the delta, split JSON lines
  with partial-trailing-line buffering, parse to the record model, merge across files by
  timestamp, hold a bounded window, and load older ranges on demand. Tests mirror the viewer
  scenarios (new record appears; partial line withheld until complete; interleave by timestamp;
  recent history on open; load older).

## 5. Log-viewer route and UI

- [x] 5.1 Add the global `logs` route (app-shell, not a session surface) and a chrome
  navigation entry. Tests: route reachable; consumes no session placement/surface.
- [x] 5.2 Render records (timestamp, severity, body, attributes, resource) with a level filter,
  free-text search, and `component` / `session.id` facets, on the 0.0.6 design tokens. Tests:
  level filter; text search; facet selection.

## 6. Verify gate

- [x] 6.1 Run `bun run verify` (format, clippy, type, lint, test) and the desktop e2e suite; fix
  to green. Confirm every spec scenario has a passing test and
  `openspec validate observability-log-viewer` passes.

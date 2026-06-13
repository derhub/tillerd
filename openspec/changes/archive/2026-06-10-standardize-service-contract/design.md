## Context

`service-host` is the standard for creating a long-lived Rust service: implement `Service { config, serve, shutdown }` and call `host::run()`, which writes the manifest, installs signal handlers, runs serve, and shuts down gracefully. Three services use it (gate, daemon-pty, mcp-gateway); more are coming, so the contract needs to scale uniformly.

Today `host::run()` also starts a `Probe` bound to a dedicated `<base>/<name>-health.sock`, answering a tiny `GET /health` with the version. The launcher never uses it: `process-launch` adopt-or-spawn reads the version from the manifest and liveness from a connect to the main control socket. So the health socket is a second socket per service that nothing depends on.

The decision is to make health an **in-process** capability of the `Service` contract — the service checks its own health — and remove the dedicated socket entirely. No health is exposed over any socket.

## Goals / Non-Goals

**Goals:**

- Make the `Service` trait the single standardized contract and growth point; add capabilities as defaulted methods.
- Add `health()` as an in-process self-check (first such capability), defaulted so existing services are untouched.
- Have the host surface health uniformly by logging it (startup + drain) — no endpoint.
- Remove the dedicated `<name>-health.sock` and its `Probe`.
- Standardize startup with `run_blocking`.

**Non-Goals:**

- Any socket/route/frame/wire protocol for health — explicitly excluded.
- Exposing health to external callers over a transport (deferred; additive later).
- Changing adopt-or-spawn (already manifest + main-socket connect).
- `contracts-rs` changes (health is in-process, not a wire type).
- `memorya`; signal/grace handling; non-Rust hosts.

## Decisions

### `health()` is an in-process trait method with a default

Add `fn health(&self) -> HealthReport` to `Service`, defaulting to `{ version: self.config().version, status: Serving }`. The service owns the logic; the default keeps the addition non-breaking. `HealthReport`/`HealthStatus` live in `service-host` alongside the trait — they are **not** wire types, because health never crosses a process boundary, so they do not belong in `contracts-rs`.

- _Alternative — health over a socket (a route or a dedicated `-health.sock`):_ rejected per the directive. Health is the service's own concern, checked in-process; no transport.

### The host surfaces health by logging it

`host::run()` calls `service.health()` and logs the status at startup (after serve begins) and during graceful drain. This gives `health()` a concrete, socket-free consumer and uniform observability across services, and it is where future surfacing (e.g. a manifest status field) would hook in if ever needed.

- _Alternative — nothing calls `health()`:_ rejected; it would be dead code. Logging is the minimal honest use.

### Remove the dedicated health socket

Delete `paths.health_socket_path()`, the `Probe`, and the `host::run()` wiring that starts/stops it. Safe because adopt never used it and health is now in-process.

### `run_blocking` entrypoint

Add `service_host::run_blocking(service)` (builds the standard multi-thread runtime, calls `host::run`, reports a startup error uniformly); the three `bin/main` entrypoints collapse to one call. Creating a service becomes *implement `Service` + one call*.

## Risks / Trade-offs

- **Health is no longer externally checkable beyond liveness.** A caller can confirm a service is alive (main-socket connect) and its version (manifest), but not its serving/draining status from outside the process. -> Accepted per the directive; status is in-process. If external status is ever needed, it is additive (a manifest field or a route) without revisiting this decision.
- **ADR-0007 names a `/health` probe** as part of the reliability contract. -> Record a new ADR revisiting the health-probe mechanism (dedicated socket probe -> in-process `Service::health()` self-check), preserving external liveness via manifest + main-socket connect; the rest of ADR-0007 stays in force.

## Migration Plan

Pre-v1, no compatibility shim. Order: (1) `service-host` `HealthReport`/`HealthStatus` + `Service::health()` default + `run_blocking`; (2) `host::run` logs health and no longer starts the `Probe`; remove `health_socket_path()` and `probe.rs`; (3) services override `health()` where meaningful; (4) collapse entrypoints to `run_blocking`; (5) docs + ADR. Rollback: revert the change.

## Open Questions

- Does any out-of-tree consumer read `<name>-health.sock`? None in-tree; assumed none.
- A future change could surface health externally (manifest status field or a route) if a real consumer appears; deferred.
- No in-force ADR is contradicted; this revisits the health-probe mechanism of ADR-0007 — the adr step records the new decision.

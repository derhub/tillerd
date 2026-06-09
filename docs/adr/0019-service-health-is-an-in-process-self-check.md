# 0019. Service health is an in-process self-check; the health socket is removed

- Status: accepted
- Date: 2026-06-09

## Context

ADR-0007 makes a `/health` liveness probe part of the reliability/operability contract. `service-host` implemented it as a dedicated per-service socket: `host::run()` binds `<base>/<name>-health.sock` and answers `GET /health` with the version. So every managed service binds two sockets — its main control socket and this health socket.

The dedicated socket earns nothing. The launcher never uses it: `process-launch` adopt-or-spawn reads the version from the manifest and liveness from a connect to the **main** control socket, not the health socket. And as the set of services grows, the goal is a standardized service contract, not more per-service sockets and bespoke health endpoints.

Health is the service's own concern: only the service knows whether it is serving, draining, or degraded. That makes health an in-process capability of the service contract, not a transport.

This ADR revisits only the health-probe **mechanism** of ADR-0007. The rest of ADR-0007 (graceful shutdown, timeouts, authenticated control plane, backpressure, plane degradation, version awareness) stays in force.

## Decision

Health becomes an in-process method of the `Service` contract, and the dedicated health socket is removed.

- The `Service` trait gains `health() -> HealthReport`, defaulting to `{ version (from config), status: serving }`; a service overrides it to report its own status (for example, draining). The service does its own health checking.
- `HealthReport` / `HealthStatus` live in `service-host` with the trait. They are **not** wire types and do **not** go in `contracts-rs`, because health never crosses a process boundary.
- There is **no** health socket, route, frame, or request protocol. `service-host` stops binding `<name>-health.sock`; `health_socket_path()` and the `Probe` are removed.
- The host surfaces health without an endpoint: `host::run()` calls `service.health()` and logs the status at startup and during graceful drain.
- External liveness checking is unchanged: version from the manifest, liveness from a connect to the main control socket. Adopt-or-spawn is untouched.
- Service creation is standardized through one `run_blocking` entrypoint, so a service is "implement `Service` + one call." The trait is the growth point: future shared capabilities are added as defaulted methods, `health()` being the first.

## Consequences

- One socket per service: the `-health.sock` family disappears from the runtime directory; discovery and binding shrink.
- Health is owned by the service and standardized by the contract; new services inherit a working default and override only when they have real status to report.
- The contract scales: adding the next shared capability is a defaulted trait method, not a new socket or endpoint per service.
- Health status is in-process only — externally, callers still get liveness (main-socket connect) and version (manifest), but not serving/draining status. If an external consumer of status ever appears, surfacing it is additive (a manifest field or a route) without revisiting this decision.
- The reliability obligation of ADR-0007 is preserved (liveness + version remain externally checkable); only the richer health transport is removed.
- Pre-v1 and breaking for anything that read `<name>-health.sock` directly (no in-tree consumer does); rollback is reverting the change.

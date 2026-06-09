## Why

More services are coming, so **creating a service must be standardized** — one contract every service implements the same way, so a new service is uniform by construction and shared concerns are handled once. The `Service` trait is that contract; this change makes it the standard's growth point: capabilities are added as defaulted trait methods (non-breaking), and **health is the first**.

Health is an **in-process self-check the service owns**, not a probe over a socket. A service answers "am I serving, and at what version?" itself; the host can surface that (logging on startup and drain) for uniform observability. There is no health socket and no health request protocol.

That also lets us drop a socket. Every long-lived service currently binds a dedicated `<name>-health.sock` that the host stands up — but the launcher never used it (adopt-or-spawn reads version from the manifest and liveness from a connect to the main control socket). With health now an in-process method, the dedicated health socket is pure redundancy and is removed.

Finally, the three services hand-roll the same runtime + `host::run` + error-exit boilerplate. A single `run_blocking` entrypoint makes creation exactly *implement `Service` + one call*.

## What Changes

- **The `Service` trait gains `health()`** returning an in-process health report (`{ version, status }`, status `serving`/`draining`). It defaults to `{ config version, serving }`, so existing services compile unchanged; a service overrides it to report its own readiness or draining. The service does its own health checking — no socket, no route, no frame.
- **The host surfaces health without a socket**: `host::run` calls `service.health()` to log the service's status (at startup and during graceful drain), giving uniform observability with no health endpoint.
- **BREAKING (pre-v1):** the dedicated per-service health socket is removed — `paths.health_socket_path()`, the `Probe`, and the host wiring that starts it all go away.
- **Adopt-or-spawn is unaffected** — it already reads version from the manifest and liveness from a connect to the main control socket; it never used the health socket.
- **Standard entrypoint.** Add `service_host::run_blocking(service)` (builds the standard runtime, runs the host, reports errors uniformly); the three `bin/main` entrypoints collapse to one call.

## Capabilities

### New Capabilities

- `service-contract`: the standard for creating a long-lived service — the `Service` trait (`config` / `serve` / `shutdown` / `health`) as the single contract and extension point, the `run_blocking` entrypoint, the rule that health is an in-process self-check (no health socket), and the host surfacing health via logging.

### Modified Capabilities

None — health does not touch any socket protocol, so no socket spec changes.

## Impact

- **`service-host`**: add `Service::health()` (with default) and a `HealthReport`/`HealthStatus` type (lives here — it is never serialized over a wire, so it is not a `contracts-rs` type); add `run_blocking`; have `host::run` log `service.health()` at startup and drain; **remove** `health_socket_path()`, the `Probe` (`probe.rs`), and the host's health-socket wiring.
- **`apps/gate`, `packages/daemon-pty`, `apps/mcp-gateway-rs`**: implement `health()` where they have meaningful status to report (otherwise inherit the default); collapse each `bin/main` to `service_host::run_blocking(X::from_env())`. No socket wiring.
- **`contracts-rs`**: no change (health is in-process, not a wire type).
- **`packages/process-launch`**: no change.
- **`docs/services.md`**: drop `<name>-health.sock`; document `Service` (incl. `health()`) + `run_blocking` as the canonical service-creation path, and health as an in-process self-check.
- **ADR**: a new ADR records moving health from a dedicated socket probe to an in-process `Service::health()` self-check, removing the health socket — revisiting the health-probe mechanism of ADR-0007 while keeping external liveness (manifest + main-socket connect) intact.
- **Out of scope**: `apps/memorya` (not a `service-host` service); exposing health to external callers over any transport (deferred — additive later via a manifest field or a route if ever needed); the mcp-gateway control-plane REST health endpoint (separate, client-facing); signal/grace handling; any non-Rust host.

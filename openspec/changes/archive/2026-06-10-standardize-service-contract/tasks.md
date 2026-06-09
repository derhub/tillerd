## 1. Service contract: health() + run_blocking (service-host)

- [x] 1.1 Add `HealthStatus` (`Serving`, `Draining`) and `HealthReport { version, status }` in `service-host` (in-process types, not wire types — not in `contracts-rs`)
- [x] 1.2 Add `fn health(&self) -> HealthReport` to the `Service` trait with a default returning `{ version: self.config().version, status: Serving }`
- [x] 1.3 Add `service_host::run_blocking(service)` that builds the standard multi-thread runtime, calls `host::run`, and reports a startup error uniformly
- [x] 1.4 Have `host::run` call `service.health()` and log the status at startup (after serve begins) and during graceful drain
- [x] 1.5 Tests: default health reports serving + configured version; an overriding service reports its own status; `host::run` logs health

## 2. Remove the dedicated health socket (service-host)

- [x] 2.1 Remove `paths.health_socket_path()`, the `Probe` (`probe.rs`), and the `host::run()` wiring that starts/stops it; update `paths`/host tests
- [x] 2.2 Confirm no code path binds `<name>-health.sock`; the host builds with no `Probe`
- [x] 2.3 Confirm `process-launch` is unaffected (still version-from-manifest + liveness-from-main-socket-connect)

## 3. Adopt the standard in each service

- [x] 3.1 Override `Service::health()` where a service has meaningful status (e.g. draining during shutdown); otherwise rely on the default
- [x] 3.2 Collapse `apps/gate/src/bin/main.rs`, `packages/daemon-pty/src/main.rs`, and `apps/mcp-gateway-rs/src/bin/gateway.rs` to `service_host::run_blocking(X::from_env())`

## 4. Docs + verification

- [x] 4.1 Update `docs/services.md`: drop `<name>-health.sock`; document `Service` (incl. in-process `health()`) + `run_blocking` as the canonical service-creation path; note health is an in-process self-check (no health socket)
- [x] 4.2 `cargo test --workspace` green, including the health-default/override and host-logs-health tests
- [x] 4.3 `bun run verify` green; manually confirm a running service binds no `-health.sock` and starts via `run_blocking`

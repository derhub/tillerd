# 0028. Service lifecycle phases (ready, drain) and manifest-based discovery

- Status: accepted
- Date: 2026-06-13

## Context

The service contract (ADR-0019 era) covers identity, version, health, and graceful
shutdown, but readiness is inferred by poll-connecting sockets (racy, cannot distinguish
starting from wedged) and there is no graceful-refusal phase distinct from shutdown.
Drain-and-restart (ADR-0029) needs a drain primitive, the orchestrator's adopt-or-spawn
needs reliable readiness, and 0.0.6 freezes the contract for the rest of 0.x — future
services must inherit these phases rather than each invent them.

## Decision

The `Service` contract in `service-host` gains two first-class lifecycle phases and a
discovery convention:

- **Ready**: the service signals readiness through a handle on its serve context once it
  is accepting work. The host records the transition in the manifest
  (`status: starting -> ready`) and logs it.
- **Drain**: SIGUSR2 puts the service into drain — refuse new work with a typed error,
  finish active work, report `Draining` health, manifest `status: draining`, exit
  cleanly when idle. SIGTERM keeps its graceful-shutdown-now meaning. Drain is a
  host-level signal, never a wire-protocol frame.
- **Discovery**: the manifest is the source of truth — name, version, pid, lifecycle
  status, socket path under the runtime layout (ADR-0025). Clients resolve a service's
  socket from its manifest only.

## Consequences

- Every service gets ready/drain for free by implementing the trait; the upgrade path,
  health indicators (0.0.8), and the E2E rig all read the same manifest lifecycle.
- Wire protocols stay free of lifecycle concerns, so the protocol freeze at 0.0.6 holds.
- The manifest schema grows `status` and `socket_path`; anything parsing manifests must
  handle them.
- Readiness now requires service cooperation (calling the handle); a service that never
  signals ready is visibly `starting` rather than silently half-up.

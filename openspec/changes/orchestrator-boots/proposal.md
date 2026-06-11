## Why

Implements roadmap **0.0.1 — "Orchestrator boots, services run"** (the first version of the 0.0.x
Foundation line).

The backend is moving from the TypeScript engine running inside the renderer to a
runtime-agnostic Rust orchestrator that each host embeds in-process (ADR-0022), with one
durable product store (ADR-0023). Before any surface can render through the new stack, the
orchestrator has to exist and boot: come up, adopt or spawn the shared services, open its
durable store, and expose an API the host binds and the SDK calls until the renderer reaches
a `ready` state. This change builds that first vertical slice — the foundation every later
0.0.x step stands on. Nothing renders yet; a blank UI that reaches `ready` is the bar.

## What Changes

- **Introduce the orchestrator as a runtime-agnostic Rust library crate (ADR-0022).** It owns
  the backend, exposes a transport-agnostic API — request/response methods plus outbound
  streams emitted over an `EventSink` trait the host implements — and is embedded in-process by
  the desktop host, which binds the API to Tauri commands and an event channel. One orchestrator
  instance per host process. Booting it drives a lifecycle that ends in a `ready` state the host
  and SDK can observe.
- **Supervise the shared services at boot.** On start the orchestrator adopt-or-spawns the gate
  and the daemon through the existing service contract (liveness from a connect to the main
  control socket, version from the manifest) and tracks per-service status using the in-process
  health self-check (ADR-0019 — no health socket). Readiness is gated on the supervised services
  being adopted/healthy. The daemon and gate themselves are unchanged; the orchestrator is their
  client and supervisor.
- **Stand up the durable product store `tillerd.db` (ADR-0023).** A rusqlite store, owned by the
  orchestrator and read/written in Rust only, created with the ADR-0023 schema by a lazy
  migration runner keyed off a `meta` schema-version row (apply pending migrations on open). The
  fixed-id "Unfiled" project is seeded. No pre-v1 data migration — the schema starts fresh; the
  throwaway `desktop-store.json` registry is not imported. Only the schema + migration runner +
  seed land here; populating session/surface rows is later (0.0.2+).
- **Make the TS `sdk` a client of the orchestrator API; turn the in-renderer engine path off.**
  **BREAKING.** The SDK gains a typed client that talks to the orchestrator API through the host
  transport; the UI reaches `ready` through it. The renderer no longer hosts the agent engine
  in-process. A blank UI is acceptable for this slice.

## Capabilities

### New Capabilities

- `orchestrator-core`: the runtime-agnostic orchestrator library crate — its boot lifecycle to a
  `ready` state, the transport-agnostic API surface (request/response + outbound streams over an
  `EventSink` the host binds), and the single-instance, embedded-in-process host-embedding
  contract.
- `orchestrator-supervision`: adopt-or-spawn of the gate and daemon at orchestrator boot via the
  service contract, per-service liveness/version tracking and in-process health (ADR-0019), and
  the readiness gate that ties service status to the orchestrator `ready` state.
- `workspace-persistence`: the `tillerd.db` rusqlite product store — the ADR-0023 schema, the
  lazy migration runner keyed by the `meta` schema version, the seeded Unfiled project, the
  two-level id ownership rule (the product `session_id` never leaves the orchestrator; `surface_id`
  is the shared kernel), and Rust-only access.
- `sdk-orchestrator-client`: the TS SDK as a typed client of the orchestrator API over the host
  transport, through which the renderer observes the orchestrator reaching `ready`.

### Modified Capabilities

- `desktop-engine-runtime`: the agent engine no longer runs inside the renderer. The desktop host
  embeds the Rust orchestrator and the renderer reaches `ready` through the orchestrator API
  instead of driving the engine and adapter in-process over injected transports.

## Impact

- **New crate:** an orchestrator library crate composing the existing `daemon-pty-client`,
  `gate-client`, `process-launch`, `service-host`, and `contracts` crates; embedded by
  `apps/desktop/src-tauri` (the `orchestrator.rs` / `bootstrap.rs` / `bridge.rs` host scaffold
  binds the API to Tauri commands + an event channel).
- **Persistence:** introduces `tillerd.db` (rusqlite) under the runtime dir with the ADR-0023
  schema and a migration runner; service-local runtime/discovery state (`daemon.json`, snapshots,
  gate registry) stays out of the DB by the persistence-model boundary.
- **SDK / UI:** `packages/sdk` adds an orchestrator-API client and the UI consumes it to reach
  `ready`; the in-renderer engine + injected daemon-transport path is turned off. Blank UI is
  acceptable.
- **Specs touched later (out of scope here):** `desktop-daemon-host` and `session-persistence`
  describe engine-era supervision and the engine-era session store; their reconciliation to the
  orchestrator model follows as those rows/flows land in 0.0.2+. This change only stands up the
  orchestrator, its supervision, the store schema, and the SDK client.
- **ADRs honored:** ADR-0019 (in-process health), ADR-0020 (surface-scoped ids), ADR-0022
  (orchestrator owns the backend; transport-agnostic API + `EventSink`; embedded library crate),
  ADR-0023 (one product store, two-level id, lazy migration).

## Context

The 0.0.x line inverts the backend from a TypeScript engine running inside the renderer to a
runtime-agnostic Rust orchestrator embedded in-process by each host (ADR-0022), backed by one
durable product store (ADR-0023). This change builds the first slice: the orchestrator crate boots,
supervises the shared services, opens its store, and exposes an API the host binds and the SDK calls
until the renderer observes `ready`. Nothing renders yet.

Current state: the shared services already exist as standalone crates and conform to the service
contract — `daemon-pty-client`, `gate-client`, `process-launch` (adopt-or-spawn), `service-host`
(the `Service` trait + in-process health, ADR-0019), and `contracts`. The desktop host
(`apps/desktop/src-tauri`) already carries a host scaffold (`orchestrator.rs`, `bootstrap.rs`,
`bridge.rs`, `gate_admin.rs`). The engine and the SDK protocol live in TypeScript
(`packages/engine`, `packages/sdk`) and drive sessions inside the renderer today.

Constraints: honor ADR-0019 (in-process health, no health socket), ADR-0020 (surface-scoped ids),
ADR-0022 (orchestrator owns the backend; transport-agnostic API + event sink; embedded library
crate), ADR-0023 (one product store, two-level id, lazy migration). The daemon and gate are
unchanged — the orchestrator is their client and supervisor. Single-user, local-first scope.

## Goals / Non-Goals

**Goals:**

- A runtime-agnostic orchestrator **library crate** that the desktop host embeds in-process and
  binds to its transport.
- A transport-agnostic API: request/response methods plus outbound streams over an event-sink the
  host implements.
- A boot lifecycle that adopt-or-spawns the gate and daemon, opens the store, and reaches an
  observable `ready` state gated on those prerequisites.
- The durable product store `tillerd.db` with the ADR-0023 schema, a lazy migration runner, and the
  seeded Unfiled project.
- A typed SDK client of the orchestrator API; the renderer reaches `ready` through it; the
  in-renderer engine path is turned off. Blank UI acceptable.

**Non-Goals:**

- Any surface rendering — terminal/agent surfaces, xterm streaming, status badges (0.0.2+).
- Populating session/surface rows or session resume (0.0.2+).
- Migrating the surface runtime / adapter into Rust (0.0.2 / 0.0.3).
- Generating the SDK wire types from `contracts` (0.1.4) — hand-author the minimal types now.
- Reconciling `desktop-daemon-host` and `session-persistence` specs to the orchestrator model —
  deferred until their rows/flows land.
- The server host (future, ADR-0022) — only the desktop host binds the API here.

## Decisions

### A new `orchestrator` library crate, not host code

The orchestrator is a standalone library crate composing `daemon-pty-client`, `gate-client`,
`process-launch`, `service-host`, and `contracts`; the desktop host depends on it and binds the API.

- **Why:** ADR-0022 mandates a runtime-agnostic crate reusable by a future server host. Putting the
  logic in `apps/desktop/src-tauri` would couple the backend to the desktop runtime.
- **Alternative considered:** implement directly in the Tauri host now, extract later — rejected;
  the extraction seam is exactly the API/event-sink boundary this slice must establish, so build it
  in the crate from the start.
- The crate is `tillerd-orchestrator` (dir `crates/orchestrator`), per the `tillerd-*` convention;
  the desktop host scaffold binds it.

### Transport-agnostic API + an event-sink trait the host implements

The orchestrator exposes request/response methods and outbound event streams. Outbound events go
through an `EventSink` trait the host implements; the desktop host binds requests to commands and
the sink to an event channel.

- **Why:** ADR-0022 — the only difference between desktop and a future server host is the host
  binary; the orchestrator must not encode the transport.
- **Alternative considered:** orchestrator owns a socket/HTTP server — rejected; that re-introduces a
  process boundary ADR-0022 explicitly removed for the embedded case.
- Scope for this slice: a `status()` query plus a streamed lifecycle event — `Booting ->
  OpeningStore -> Supervising -> Ready`, with a terminal `Failed { reason }` — so the host and SDK can
  show boot progress and failure (the typed signal 0.1.3 first-run UX builds on). Richer domain
  methods arrive with the surface runtime.

### Boot lifecycle with an explicit `ready` gate

Boot is a defined sequence: open + migrate the store -> adopt-or-spawn and health-check the gate and
daemon -> reach `ready`. The orchestrator exposes the current lifecycle state and emits transitions
over the event sink; it never reports `ready` until the store is open and both services are
available, and a failed prerequisite surfaces a typed error instead of a false `ready`.

- **Why:** the host and SDK need one observable signal that the backend is usable; readiness must be
  honest so the UI can show failure states later (0.1.3).
- **Alternative considered:** implicit readiness (first successful call) — rejected; gives no
  distinct boot-failure surface and races with service spawn.

### Supervision reuses the service contract and in-process health

Adopt-or-spawn uses `process-launch` (liveness from a control-socket connect, version from the
manifest); per-service status uses the `service-host` in-process health self-check. No health
socket (ADR-0019).

- **Why:** the contract and adopt-or-spawn already exist; this slice composes them rather than
  inventing supervision.
- **Alternative considered:** a bespoke supervisor with its own probes — rejected; duplicates the
  contract and violates ADR-0019.

### Persistence: one embedded store, lazy forward-only migrations

`tillerd.db` (at `~/.tillerd/tillerd.db`, flat at the runtime-dir root) is opened on boot; a `meta`
record holds the schema version; pending migrations apply in order to reach the version the binary
expects; a store newer than the binary is refused with a typed
error. The schema is exactly ADR-0023. The fixed-id Unfiled project is seeded. No pre-v1 data
migration — `desktop-store.json` is throwaway and not imported. Service-local state
(`daemon.json`, snapshots, the gate registry) stays out of the store (the persistence-model
boundary).

- **Why:** ADR-0023; lazy vN->vN+1 on open keeps migration simple and matches the launch-spec
  approach (ADR-0021).
- **Alternative considered:** a migrations framework / external tool — rejected for a single-file,
  single-writer local store; embedded ordered migrations are enough and keep the runner in-crate.
- Migrations are append-only and each is covered by a fresh-init test and an upgrade test.

### Persistence is a repository trait behind a `persistence` module; crate split deferred

Persistence lives as a `persistence` module inside the orchestrator crate, exposed to the rest of
the crate as a repository `Store` trait over row/domain types. All SQL is confined to the trait's
implementation; domain, boot, and supervision code depend on the trait, never on the persistence
implementation or any SQL. The `persistence` layer remains a module, not a separate crate, for this
slice.

- **Why:** the boundary that matters is "domain logic never touches SQL," which a trait + module
  privacy enforces inside one crate. ADR-0022 lists persistence as a layer the orchestrator crate
  owns, so one crate is the decided packaging; a trait keeps the seam clean without speculative
  structure.
- **Why a trait specifically:** it makes the store fakeable in tests (boot/readiness tested against
  an in-memory fake, no real store), localizes the eventual rusqlite->other-backend swap, and makes
  promoting the module to its own crate a near-mechanical move if a trigger appears.
- **Alternative considered — a separate `persistence` crate now:** rejected as premature. It buys
  only compile-parallelism and compiler-enforced dependency direction, neither pressing at this
  size, and would force freezing a repository API before the real query patterns (sessions,
  surfaces, resume) exist in 0.0.2–0.0.4.
- **Revisit the crate split when** rusqlite compile time drags iteration, a second consumer needs
  the store without the orchestrator (e.g. a migration/inspection tool), or the schema stabilizes
  (~0.0.4) and the backend-swap option is worth insuring. The split is then: move the module to a
  crate, no API redesign.

### SDK becomes an orchestrator-API client; engine path off

`packages/sdk` gains a typed client that calls the orchestrator API over the desktop transport and
subscribes to its status stream; the renderer reaches `ready` through it. The in-renderer engine
construction is removed from the desktop path. Wire types are hand-authored and minimal for this
slice (status/readiness only); generating them from `contracts` is 0.1.4.

- **Why:** ADR-0022 — TypeScript is UI and SDK only.
- **Alternative considered:** keep the engine path behind a flag — rejected; pre-v1 allows a clean
  break and a dual path would entangle the inversion.

## Risks / Trade-offs

- **[Hand-authored wire types drift from the Rust API before 0.1.4]** -> keep the API surface tiny
  (status + readiness), centralize the types in one SDK module, and regenerate from `contracts` at
  0.1.4; the smaller the surface now, the smaller the reconciliation.
- **[Adopt-or-spawn race if a stale or half-up service is present]** -> rely on the existing
  control-socket connect as the liveness arbiter and manifest version for compatibility; single-user
  local scope makes concurrent hosts unlikely, and `process-launch` already owns this.
- **[Turning the engine path off yields a non-functional UI]** -> accepted for this slice; a blank UI
  that reaches `ready` is the bar, and the `desktop-engine-runtime` delta removes the obsolete
  in-renderer requirements so the contradiction does not linger.
- **[Migration runner corrupts the store on a bad migration]** -> forward-only, append-only
  migrations, each tested fresh + upgrade; refuse a store newer than the binary rather than guess.
- **[Readiness deadlock if a service never comes up]** -> boot surfaces a typed failure rather than
  blocking forever; first-run failure UX is 0.1.3, but the typed signal exists now.

## Migration Plan

- Pre-v1, breaking: no data migration. The throwaway `desktop-store.json` registry is not imported;
  `tillerd.db` starts fresh with a seeded Unfiled project.
- Land the orchestrator crate and the store/migration runner first (no host wiring needed to test
  boot, supervision, and migrations in isolation).
- Wire the desktop host to embed the orchestrator and bind the API/event-sink; add the SDK client;
  remove the in-renderer engine construction.
- Rollback is reverting the crate wiring and re-enabling the engine path; acceptable pre-v1 but not a
  supported runtime toggle.

## Open Questions

All planning-level questions are resolved:

- **Crate:** `tillerd-orchestrator` (dir `crates/orchestrator`); the desktop host scaffold binds it.
- **Persistence packaging:** a `persistence` module inside the crate behind the `Store` trait; crate
  split deferred to a trigger (rusqlite compile drag / second consumer / ~0.0.4 schema-stable).
- **Status API:** a `status()` query plus a streamed lifecycle event (`Booting -> OpeningStore ->
  Supervising -> Ready`, terminal `Failed { reason }`).
- **Store path:** `~/.tillerd/tillerd.db`, flat at the runtime-dir root.

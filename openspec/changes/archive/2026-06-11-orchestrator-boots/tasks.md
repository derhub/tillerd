## 1. Orchestrator crate scaffold

- [x] 1.1 Add a new runtime-agnostic `tillerd-orchestrator` library crate (dir `crates/orchestrator`) as a workspace member, depending on `daemon-pty-client`, `gate-client`, `process-launch`, `service-host`, and `contracts`; no host-runtime or UI deps (orchestrator-core: runtime-agnostic embeddable library; ADR-0022).
- [x] 1.2 Define the typed error type for the crate (boot failures, store-version mismatch, service-unavailable) used across persistence, supervision, and boot (orchestrator-core: typed boot-failure surface).

## 2. Workspace persistence (`tillerd.db`)

- [x] 2.1 Define the `persistence` module's repository `Store` trait plus its row/domain types; confine all SQL to the trait implementation so domain, boot, and supervision depend only on the trait (design: repository-trait seam). Provide an in-memory fake implementation for use by other crates' tests.
- [x] 2.2 Write failing tests: a fresh store initializes to the current schema version and records it in `meta` (workspace-persistence: schema version + lazy migration runner; ADR-0023).
- [x] 2.3 Implement the embedded store open + create behind the trait at `~/.tillerd/tillerd.db`, Rust-only access; no service-local state in the store (workspace-persistence: single durable product store).
- [x] 2.4 Implement the ADR-0023 schema as ordered migrations: projects, worktrees, launch templates, sessions, surfaces, commands, secret references, settings, and the `meta` schema-version record (workspace-persistence: product schema).
- [x] 2.5 Implement the lazy migration runner: apply pending migrations in order on open; refuse (typed error) a store whose version is newer than the binary supports (workspace-persistence: schema version + lazy migration runner). Add an upgrade-path test (older store migrated forward) and a newer-store-refused test.
- [x] 2.6 Seed the fixed-id "Unfiled" project on initialization; test a session without an explicit project resolves to it (workspace-persistence: seeded default project).
- [x] 2.7 Encode the two-level id ownership rule: mint and store `session_id` only in the store; expose `surface_id` as the shared identifier; test the product `session_id` is never handed to a backend call (workspace-persistence: two-level id ownership; ADR-0020/0023).

## 3. Service supervision

- [x] 3.1 Write failing tests for adopt-or-spawn: an already-running compatible service is adopted (no duplicate spawn); an absent service is spawned (orchestrator-supervision: adopt-or-spawn at boot).
- [x] 3.2 Implement adopt-or-spawn of the gate and daemon via `process-launch` and the service contract — liveness from a control-socket connect, version from the manifest (orchestrator-supervision: adopt-or-spawn at boot).
- [x] 3.3 Track per-service status via the `service-host` in-process health self-check (no health socket); expose liveness + version per service (orchestrator-supervision: per-service status via in-process health; ADR-0019).
- [x] 3.4 Implement the readiness gate: services-available is true only when both the gate and daemon are adopted/spawned and report available; a service that cannot be made available yields a typed failure (orchestrator-supervision: readiness gated on services).

## 4. Orchestrator boot, API, and event sink

- [x] 4.1 Define the `EventSink` trait the host implements and the transport-agnostic API for this slice: a `status()` request method and a streamed lifecycle event — `Booting -> OpeningStore -> Supervising -> Ready` with a terminal `Failed { reason }` (orchestrator-core: transport-agnostic API surface; ADR-0022).
- [x] 4.2 Write failing tests for the boot lifecycle using a fake `EventSink` and the in-memory fake `Store` (2.1): boot opens the store and supervises services, then reaches `ready`; `ready` is not reported until store-open and services-available both hold (orchestrator-core: boot lifecycle reaches an observable ready state).
- [x] 4.3 Implement the boot sequence (open+migrate store -> supervise services -> ready), emit the lifecycle transitions (`Booting -> OpeningStore -> Supervising -> Ready`) over the `EventSink`, and expose the current state via `status()`; a failed prerequisite emits terminal `Failed { reason }` (typed) and does not report `ready` (orchestrator-core: boot lifecycle / readiness gate).
- [x] 4.4 Assert single-instance, embedded-in-process construction (one instance owns the backend per host process) (orchestrator-core: runtime-agnostic embeddable library).

## 5. Desktop host embedding

- [x] 5.1 Embed the orchestrator crate in `apps/desktop/src-tauri`; construct one instance at host startup and bind its request methods to host commands (orchestrator-core: embedded in-process by a host).
- [x] 5.2 Implement the host `EventSink` binding the orchestrator status stream to the host event channel (orchestrator-core: host binds the event sink).
- [x] 5.3 Verify the host reaches and exposes `ready` after boot (manual/integration: host emits a `ready` status event once the orchestrator is ready).

## 6. SDK client and engine-path removal

- [x] 6.1 Write failing tests for the SDK orchestrator client: a typed request method routes to the orchestrator API over the host transport and returns a typed result; subscription receives status events (sdk-orchestrator-client: SDK is a typed client of the orchestrator API).
- [x] 6.2 Implement the typed SDK orchestrator client (status/readiness request + status event subscription), with hand-authored minimal wire types centralized in one module (sdk-orchestrator-client; wire-type generation deferred to 0.1.4 per design).
- [x] 6.3 Remove the in-renderer engine construction from the desktop path; route all backend interaction through the SDK client (sdk-orchestrator-client: in-renderer engine path disabled; desktop-engine-runtime delta — REMOVED in-renderer requirements).
- [x] 6.4 Make the renderer reach a usable state by observing `ready` through the SDK client; a blank UI that reaches `ready` is acceptable; reflect not-ready/boot-failure otherwise (sdk-orchestrator-client: renderer reaches readiness through the client).

## 7. Verification

- [x] 7.1 End-to-end: launch the desktop host with no services running -> orchestrator spawns gate + daemon, opens a fresh `tillerd.db` with the Unfiled seed, and the blank renderer observes `ready`.
- [x] 7.2 Re-launch with services already running -> orchestrator adopts them (no duplicate spawn) and reaches `ready`.
- [x] 7.3 Run the workspace checks (build, test, lint, format) green for the new crate, the host wiring, and the SDK client.

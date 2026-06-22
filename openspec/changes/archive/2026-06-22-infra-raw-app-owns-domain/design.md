## Context

ADR-0036 split storage into pure `entities/`, per-entity `infra/` repos + the surface runtime, and a CQS `app/` layer. In practice domain logic still sits in `infra/`. An audit of every file under `crates/orchestrator/src/infra/` classified each as raw (correct) or domain-leak (should move). Six leaks were found; the rest is clean raw I/O. Separately, two in-flight lint rules (`entities-app-or-infra-only`, `infra-only-in-app`, currently `warning`) push the same model: `app/` is the only layer that names entities/infra and the only owner of domain rules. This change moves the leaks so those rules can eventually flip to `error`.

In-force ADRs: 0033 (two-plane storage), 0034 (state model as contract), 0036 (de-abstracted storage), 0037 (event dispatch). This change refines 0036 (infra was allowed to own `Row -> Entity` and some rules); it is recorded as a superseding ADR, not by editing 0036.

## Goals / Non-Goals

**Goals:**

- `infra/` is a raw API: query execute/bind, column↔field mapping, socket I/O, frame codec, fs I/O — nothing else.
- `app/` owns every domain rule and is the sole integrator of entities + infra.
- `infra/` exposes concrete raw types named by app — no remaining port/trait abstraction over a single I/O target. The surface runtime is a concrete `DaemonPtyApi` (`infra/daemon_pty_api/`), not an `Arc<dyn SurfaceRuntime>`.
- Behavior is preserved; each move is covered by the existing bus-level test (the contract lives at app already).

**Non-Goals:**

- No new feature or wire/format change. No change to entity shapes beyond a normalization constructor + canonical id constants.
- Not touching the `daemon-pty-client` crate (raw wire codec, below infra) or the daemon-socket wire protocol — the runtime restructure (D5) is a rename + port removal, not a transport rewrite.
- Not touching session `status↔archived_at` mapping (raw column mapping) or the migration seeds (raw, with the id constant owned by entities).
- Not flipping the `*-only-in-app` rules to `error` yet (separate step once the tree is clean).
- The surface output streaming mechanism (push sink → `recv()` raw source / `events/` / pump) is out of scope here; the existing push sink keeps working. It is converted by `standardize-event-dispatch`, which lands after this change and builds on the now-raw, kind-agnostic daemon.
- De-domaining the runtime id (`SurfaceId` → `contracts::SessionId`) so infra is fully wire-only is out of scope here — it is entangled with the sink param type and owned by `standardize-event-dispatch` (see D5). This change leaves `DaemonPtyApi` on `SurfaceId`.

## Decisions

### D1 — The move pattern

Every leak follows one shape: an infra method that does **load → CHECK RULE → act** splits into (a) the bare load + bare act in infra (get/list/delete/get_all), and (b) the rule in the app handler (load → check → act). The bus-level behavior test is unchanged and stays green — it asserts the contract through `bus`, not the layer, so relocation is invisible to it. *Alternative rejected:* a "domain service" struct between app and infra — that just relocates the leak; the handler is the domain owner.

### D2 — The six moves

1. **`config/setting.rs` resolve/resolve_all → app.** infra keeps raw `get(scope, key)` and `get_all(scope)`. `app/settings/resolve_setting` does the project-over-global cascade; `resolve_settings` loads both scopes, merges, sorts.
2. **`config/theme.rs` discard guard → app.** infra exposes raw `delete(id)`. `app/settings/discard_theme` does `get` → reject `Prebuilt` → `delete`.
3. **`notification.rs` prune → app.** infra runs the `DELETE` it is given; `app/notification` owns the retention count.
4. **`runtime/daemon.rs` kind guard → app.** `SpawnRequest` drops `kind`; the daemon spawns a PTY for any request. `app/surface/spawn_surface` rejects non-`Terminal` (the 0.x terminal-only gate) **before** persisting, so no `pending`→`failed` row is written. The duplicate-proxy guard stays in infra (raw integrity of its `proxies` map).
5. **`surface_repo.rs` initial status + ordering → app.** `create` takes the initial status instead of hardcoding `Pending`; the live-first sort becomes an app-owned ordering (named constant) rather than a `CASE` baked into the repo query.
6. **`project.rs` name normalization → entities/app.** `name.trim()` moves to `Project::new` (or the create handler), removing the store/return asymmetry where the persisted value differs from the returned one.
7. **`config/keybinding.rs` precedence → app.** infra keeps raw `defaults()` / `overrides()` reads. The override-wins cascade (`resolve`), the defaults+overrides merge (`list`), and the default-chord shadowing rule (`resolve_action`, "a default chord is suppressed when its action is overridden") move to app handlers — the same shape as move 1. Found by re-audit: keybinding is a near-duplicate of `setting.rs`, missed in the first pass; moving one precedence engine and not its twin would leave the boundary half-applied.
8. **`session.rs` list ordering → app.** the `ORDER BY pinned DESC, sort_order ASC` (pinned-float) is a semantic UX precedence, the same class as move 5's live-first sort; it becomes an app-owned ordering constant. The raw `list` returns rows; the handler applies the order.

### D3 — What stays raw

`session.rs` encodes status into the `archived_at` column and decodes it back — that is column↔field mapping, infra's job. The migration seeds the `Default` workspace and `Unfiled` project; the canonical ids are constants owned by `entities` (`WorkspaceId::default` / `ProjectId::unfiled`) and the migration seeds those values. Clean repos (`command`, `launch_template`, `workspace`, `migrate`) are untouched; `config/keybinding` loses its precedence (move 7) but its remaining reads are raw. **Ordering split:** a *deterministic lexical* sort (`setting`/`theme`/`profile`/`keybinding` `list` ordered by key/id) is raw — stable output of a query, no product decision. A *semantic precedence* sort (live-first in move 5, pinned-float in move 8) encodes a UX rule and is domain. Only the latter moves. The `theme`/`profile` `get_active` pointer-then-load and `profile.duplicate` field copy are raw compositions (no rule checked) and stay. Separately, `surface_repo` create id-minting and `project` create initial-state defaults are owned by the `client-assigned-create-ids` change (handlers build the full entity), not duplicated here. The runtime's `transport`/`fake` move verbatim under the rename (D5); only its `mod.rs` changes (the port is removed). The **`daemon-pty-client` crate** is already pure raw wire codec (frame encode/decode, `SessionFrame`, request encoders) — the layer *below* infra that the daemon transport consumes — and is left untouched; it is not part of `infra/` and is not a leak.

### D4 — Daemon becomes kind-agnostic

The runtime's only domain knowledge was "only Terminal launches." Removing `kind` from `SpawnRequest` makes the daemon a pure "spawn a PTY for this surface with these params"; surface-kind capability lives entirely in app. This also tightens behavior: an unsupported kind is rejected up front with a validation error and leaves no persisted trace.

### D5 — Runtime becomes a concrete raw API (`daemon_pty_api`), port removed

Once the daemon is kind-agnostic (D4) its only remaining job is raw daemon-socket I/O. The `SurfaceRuntime` trait + `Arc<dyn SurfaceRuntime>` dynamic dispatch is the last abstraction in `infra/runtime/`; the de-abstraction theme (ADR-0036/0038: infra is a concrete raw API named by app) says it goes. Rename `infra/runtime/` → `infra/daemon_pty_api/`; promote `DaemonRuntime` to a concrete `DaemonPtyApi` (same eight inherent methods); keep `SurfaceEventSink` (the output port — a genuine multi-impl boundary, untouched).

The trait is also the app-wide test seam: `FakeRuntime` is injected as `Arc<dyn SurfaceRuntime>` into `Ctx` at ~12 `boot`/`test_util` sites, and the bus tests assert through its `RuntimeCall` log. To remove the trait without rewriting those assertions, `Ctx` holds a `Runtime` enum `{ Daemon(DaemonPtyApi), Fake(FakeRuntime) }` with inherent methods delegating per variant (static dispatch). `FakeRuntime` stays as-is; the swap is purely `Arc<dyn> → Runtime::Fake`. *Alternative rejected:* delete `FakeRuntime` and fake the socket (`DaemonConnection`) so prod and test share one `DaemonPtyApi` — cleaner in theory but rewrites every test's assertion surface from `RuntimeCall` to recorded wire frames, breaking the "behavior preserved, bus tests stay green" guarantee this change rests on. The enum keeps two concrete impls, which is acceptable: both are real (prod + test double), so it is not an ornamental one-impl trait.

**Scoped out — the id vocabulary.** `DaemonPtyApi` still names `entities::SurfaceId` (method params, the `proxies` key, the `SurfaceEventSink` callbacks), so infra is not yet *fully* wire-only — a truly raw daemon client would speak `contracts::SessionId` (the wire id is the surface-id string, `SessionId(surface.as_str())`, a lossless derivation app can own). This is not fixed here because the only lever forcing `SurfaceId` into infra is the sink's parameter type: flip the sink to wire ids and the reverse-map vanishes — but the sink is rewritten by `standardize-event-dispatch` (`SurfaceEvents` → `SurfaceSink`/`SurfaceEvent<'_>`, with the `boot` `Bridge` as the `SessionId ↔ SurfaceId` translation point). De-domaining the id piecemeal here would need an interim reverse registry and collide with that rewrite. So the id flip is owned by `standardize-event-dispatch`; this change ships the port de-abstraction (trait → concrete enum) and leaves the id vocabulary as the one documented remaining infra-names-domain item.

## Risks / Trade-offs

- **A relocated rule changes an error code/timing** (daemon kind rejection → app validation error) → acceptable behavior refinement; update the one test that asserts the old error and document it.
- **Moving the live-first sort to app could scatter ordering** → keep it a single named ordering constant referenced by the list handler, not duplicated.
- **`create` signature churn (initial status, normalized name)** → callers are in-crate (app) and updated in the same change; no external surface.
- **Port removal (D5) touches ~12 `Ctx`-construction sites** → all in-crate (`boot` + `app/*/test_util`); the compiler enumerates them and the `Runtime::Fake` swap is mechanical. Risk it collides with `standardize-event-dispatch` (next change, same module): bounded — that change reworks the output sink/streaming path, not the `Runtime` dispatch shape, so they touch different lines of `daemon_pty_api/`.
- **Partial migration leaves mixed styles** → each move is a self-contained task with its green bus test; order is independent.

## Migration Plan

Per-leak, independently: strip infra to raw → lift the rule into the app handler/entity → run the existing bus test (must stay green) → adjust only tests that assert a relocated error. Then `bun run verify` + `bun run e2e`, and `ast-grep scan` (the `*-only-in-app` warnings should drop as references leave non-app code). Rollback is per-move (revert the file pair). No data or wire-format migration.

## Open Questions

- After the moves, are the `entities-app-or-infra-only` / `infra-only-in-app` warning counts low enough to flip to `error` in this change, or as a follow-up?
- `surface_repo.create` taking an initial status vs always `Pending` + an explicit app transition — confirm the simplest call shape.
- This change suggests superseding ADR-0036's "repos own `Row -> Entity` and some rules" clause; the adr step records ADR-0038. No other in-force ADR conflicts.

## Appendix: D5 target shape (illustrative)

Signatures grounded in the current `daemon.rs`; method bodies elided — they promote 1:1 from the existing private fns (`stop_surface`, `close_surface`, `list_running`, `send_input`, `send_resize`, `attach_proxy`, `drop_proxy`). The id type stays `SurfaceId` here (the wire-only flip is owned by `standardize-event-dispatch`, see D5). The artifacts (D5 + tasks 9.1–9.4) are the source of truth; if real code forces a tweak, the artifact wins.

```rust
// infra/daemon_pty_api/mod.rs  (was infra/runtime/mod.rs)

mod daemon;
mod fake;
mod transport;

pub use daemon::{DaemonPtyApi, ResolvedCommand};   // was DaemonRuntime
pub use fake::{FakeRuntime, RuntimeCall};
pub use transport::{default_daemon_socket, DaemonConnection, TransportError};

use std::sync::Arc;
use crate::entities::SurfaceId;
use crate::shared::Result;

// SurfaceEventSink, SpawnRequest (now kind-less), Geometry, SpawnCommand stay here.
// SurfaceRuntime trait + BoxFut/Pin/Future: DELETED.

/// The surface runtime the composition root holds. Static dispatch over the two
/// concrete impls — no `Arc<dyn>`, no boxed futures.
pub enum Runtime {
    Daemon(DaemonPtyApi),
    Fake(FakeRuntime),
}

impl Runtime {
    pub async fn spawn(&self, request: SpawnRequest) -> Result<()> {
        match self { Self::Daemon(r) => r.spawn(request).await, Self::Fake(r) => r.spawn(request).await }
    }
    pub async fn stop(&self, surface: &SurfaceId) -> Result<()> {
        match self { Self::Daemon(r) => r.stop(surface).await, Self::Fake(r) => r.stop(surface).await }
    }
    pub async fn close(&self, surface: &SurfaceId) -> Result<()> {
        match self { Self::Daemon(r) => r.close(surface).await, Self::Fake(r) => r.close(surface).await }
    }
    pub async fn list(&self) -> Result<Vec<SurfaceId>> {
        match self { Self::Daemon(r) => r.list().await, Self::Fake(r) => r.list().await }
    }
    pub async fn input(&self, surface: &SurfaceId, bytes: &[u8]) -> Result<()> {
        match self { Self::Daemon(r) => r.input(surface, bytes).await, Self::Fake(r) => r.input(surface, bytes).await }
    }
    pub async fn resize(&self, surface: &SurfaceId, cols: u16, rows: u16) -> Result<()> {
        match self { Self::Daemon(r) => r.resize(surface, cols, rows).await, Self::Fake(r) => r.resize(surface, cols, rows).await }
    }
    pub async fn attach(&self, surface: &SurfaceId) -> Result<()> {
        match self { Self::Daemon(r) => r.attach(surface).await, Self::Fake(r) => r.attach(surface).await }
    }
    pub async fn detach(&self, surface: &SurfaceId) -> Result<()> {
        match self { Self::Daemon(r) => r.detach(surface).await, Self::Fake(r) => r.detach(surface).await }
    }
}
```

```rust
// infra/daemon_pty_api/daemon.rs  (was DaemonRuntime)

pub struct DaemonPtyApi {
    sink: Arc<dyn SurfaceEventSink>,   // output port stays a trait (real multi-renderer boundary, ADR-0037)
    socket: PathBuf,
    proxies: Mutex<HashMap<SurfaceId, TerminalProxy>>,
}

impl DaemonPtyApi {
    pub fn new(sink: Arc<dyn SurfaceEventSink>, socket: PathBuf) -> Self {
        Self { sink, socket, proxies: Mutex::new(HashMap::new()) }
    }

    // Inherent async API — was the `impl SurfaceRuntime` body, now public directly.
    // No Box::pin, no BoxFut. The eight methods promote 1:1 from the old private fns.
    pub async fn spawn(&self, request: SpawnRequest) -> Result<()> {
        let SpawnRequest { surface, command, token, geometry, cwd } = request; // `kind` GONE (move 4)

        // RAW integrity guard stays — protects infra's own proxies map.
        if self.proxies.lock().await.contains_key(&surface) {
            return Err(surface_err(&surface, "surface already has a proxy"));
        }
        // NO `kind != Terminal` check — that capability rule now lives in
        // app/surface/spawn_surface, rejected before persist.

        let wire = wire_id(&surface);
        let (conn, rx) = DaemonConnection::connect(&self.socket).await
            .map_err(|e| surface_err(&surface, e))?;
        // ... unchanged spawn over the socket ...
        Ok(())
    }

    pub async fn stop(&self, surface: &SurfaceId)   -> Result<()> { /* was stop_surface  */ }
    pub async fn close(&self, surface: &SurfaceId)  -> Result<()> { /* was close_surface */ }
    pub async fn list(&self)                        -> Result<Vec<SurfaceId>> { /* was list_running */ }
    pub async fn input(&self, surface: &SurfaceId, bytes: &[u8]) -> Result<()> { /* was send_input */ }
    pub async fn resize(&self, surface: &SurfaceId, cols: u16, rows: u16) -> Result<()> { /* was send_resize */ }
    pub async fn attach(&self, surface: &SurfaceId) -> Result<()> { /* was attach_proxy */ }
    pub async fn detach(&self, surface: &SurfaceId) -> Result<()> { /* was drop_proxy */ }
}
```

```rust
// context.rs

use crate::infra::daemon_pty_api::Runtime;   // was: infra::runtime::SurfaceRuntime

pub struct Ctx {
    // ...
    runtime: Runtime,                         // was: Arc<dyn SurfaceRuntime>
}

impl Ctx {
    pub fn runtime(&self) -> &Runtime { &self.runtime }   // was: &dyn SurfaceRuntime
}
```

```rust
// boot.rs / app/*/test_util.rs — the ~12 injection sites

// before: Arc::new(FakeRuntime::new()) as Arc<dyn SurfaceRuntime>
let runtime = Runtime::Fake(FakeRuntime::new());

// prod boot — before: Arc::new(DaemonRuntime::new(sink, socket)) as Arc<dyn SurfaceRuntime>
let runtime = Runtime::Daemon(DaemonPtyApi::new(sink, socket));
```

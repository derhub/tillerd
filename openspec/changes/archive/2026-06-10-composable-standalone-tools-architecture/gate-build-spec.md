# Gate Build Spec (tasks 4.1 + 5.1–5.14)

Synthesized from the `gate-build-spec` design workflow (6 agents, 2026-06-07), grounded in the real `contracts-rs` / `service-host` APIs. TDD-first, inline build (worktree-parallel is blocked: agents branch from `main` which lacks `contracts-rs`/`service-host`). One commit per `tasks.md` item.

## Crate identity

- Dir `apps/gate`; Cargo package `athing-gate`; lib `athing_gate`; bin `athing-gate`; npm `@athing/gate`.
- bin **+** lib crate. `#![forbid(unsafe_code)]` in lib.rs; **no** `#![deny(missing_docs)]` (internal framework, not a published API).
- Add to root `Cargo.toml` members in the scaffold commit; add `package.json` (turbo, binary pattern: `bin` → `../../target/release/athing-gate`, `dev`), `.gitignore` (`/target`). No per-crate `Cargo.lock`, no per-crate `[profile]` (workspace owns both).

## Resolved decisions (accepted from design openDecisions)

- **D1 constant-time:** `subtle::ConstantTimeEq` over equal-length token hashes (no daemon reference exists). Used by `Auth` and the admin surface.
- **D2 hook transport:** loopback HTTP over **TCP `127.0.0.1`** via `axum`; the agent hook `curl`s the URL injected as `ATHING_GATE_URL` (orchestrator 7.4). Gate binds `ATHING_GATE_PORT` (default `0` = ephemeral) and writes the actual `http://127.0.0.1:<port>` to `<base>/gate.url` so the orchestrator can publish it. `axum` `DefaultBodyLimit` = the OOM cap (`ATHING_GATE_HOOK_MAX_BODY`, sane default e.g. 1 MiB).
- **D3 concurrency:** `std::sync::Mutex<HashMap<..>>` for `SessionRegistry`; `tokio::sync::broadcast` per session for subscriptions. No `dashmap`.
- **D4 droppedN:** per-session counter incremented from `broadcast::error::RecvError::Lagged(n)` (drop-oldest is broadcast's native behavior); reported as the session's `droppedN`, logged once per lag.
- **D5 subscribe face:** dedicated `endpoint/subscribe.rs` on its own loopback Unix socket (consumed by `gate-client`, task 6.2). Distinct from the hook face.
- **D6 health:** **no** gate health endpoint — rely entirely on the service-host `Probe` (`health_socket_path`, unauthenticated, version-carrying).
- **D7 adapter signature:** sync, pure, dyn-safe: `fn parse_hook(&self, body: &[u8]) -> Result<HookEvent, ParseError>`. `Normalize` maps `ParseError -> Reject::Invalid`.
- **D8 allow-policy:** v1 allow-all (no on-disk policy file, R4); `Reject::Denied` reserved/unused so policy middleware lands later with no contract change.
- **D9 framing:** reimplement the trivial length-prefix codec locally in `endpoint/mod.rs`; do **not** import `daemon-pty-client` (8.4: only it knows the PTY wire — these are different wires anyway).
- **D10 Service impl** lives in `service.rs`; thin `bin/main.rs`.
- **D11 Middleware** is `#[async_trait]` (object-safe `Arc<dyn Middleware>`); `AgentAdapter` is a sync trait; the `Service` impl uses service-host's native async-in-trait (not async_trait).

## Cargo deps

- `contracts-rs = { path = "../../packages/contracts-rs" }` — `HookEvent`, `HookKind`, `SessionId`, `CorrelationId`, `ToolInbound`, `HookSubscribeRequest`, `HOOK_SUBSCRIPTION_WIRE_VERSION`.
- `service-host = { path = "../../packages/service-host" }` — `host::run`, `Service`, `ServiceConfig`, `ServeContext`, `Paths`, `ChildRegistry` (+ liveness Probe = the health face).
- `tokio = { version = "1", features = ["rt-multi-thread","macros","net","sync","io-util","time","signal"] }`.
- `async-trait = "0.1"` — object-safe `Middleware`.
- `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`.
- `thiserror = "2"` — `Reject`, `ParseError`, `PublishError`.
- `tracing = "0.1"` — observation records + bound context (ADR-0012).
- `uuid = { version = "1", features = ["v4"] }` — assign `CorrelationId` when absent.
- `bytes = "1"` — `Ctx.body` / `Outbound::Forward(Bytes)`.
- `subtle = "2"` — constant-time token compare.
- `axum = "0.8"` — hook HTTP ingress (workspace precedent: mcp-gateway-rs).
- dev: `tokio` test-util/macros/rt-multi-thread; `tempfile = "3"`.

## Canonical framework types (lib.rs / middleware/mod.rs)

```rust
pub enum Kind { Hook, ToolCall, ToolResult }            // Clone+Copy+Eq
pub struct Ctx { pub kind: Kind, pub session: SessionId, pub correlation: CorrelationId,
                 pub token: Token, pub body: Bytes, pub event: Option<HookEvent> }
pub enum Outbound { Accepted, Forward(Bytes) }
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Reject { Unauthenticated, Invalid(String), Denied(String) }
pub type Flow = Result<Outbound, Reject>;
pub struct Token(String);                                // constant-time compare
pub struct Next<'a> { /* remaining chain + terminal */ } // single-use
impl<'a> Next<'a> { pub async fn run(self, ctx: Ctx) -> Flow; pub fn noop() -> Self; pub fn spy() -> (Self, Arc<Mutex<bool>>) }
#[async_trait] pub trait Middleware: Send + Sync { async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow }
pub fn seq(items: Vec<Arc<dyn Middleware>>) -> Arc<dyn Middleware>;   // short-circuit on first Reject/terminal Outbound
pub fn par(items: Vec<Arc<dyn Middleware>>) -> Arc<dyn Middleware>;   // join-all, panic-isolated
pub struct Inbound { pub kind: Kind, pub session: SessionId, pub correlation: Option<CorrelationId>, pub token: Token, pub body: Bytes }
pub struct Router { /* globals [Observe,Auth]; routes: HashMap<Kind, Arc<dyn Middleware>> */ }
impl Router { pub async fn handle(&self, inbound: Inbound) -> Flow }  // assigns CorrelationId once when absent
```

Middleware shapes (D3): GATE (Auth), AROUND (Observe), TRANSFORM (Normalize), TERMINAL (FanOut, PassThrough). `Ctx` moves into `next.run(ctx)`; copy session/correlation/kind before calling next if needed after.

Key per-area types: `Auth{registry}`; `Observe{sink}` + `ObserveSink` trait + `ObservationRecord{ts,session_id,correlation_id,component:"gate",kind,event_type?,outcome,latency_ms,fanout_n?,dropped_n?}` + `RecordOutcome{Accepted,Rejected(String)}`; `Normalize{adapter: Arc<dyn AgentAdapter>}`; `AgentAdapter::parse_hook` (sync) + `ParseError{InvalidJson,MissingField}`; `Subscriptions` (per-session `broadcast::Sender<HookEvent>`, cap 256, drop-oldest, dropped counter) `subscribe/publish/end`; `SessionRegistry` (`Mutex<HashMap<SessionId,{token_hash,allow_policy}>>`) `register/deregister/verify`.

## Build order (one committable unit per step; cargo test -p athing-gate + clippy --all-targets --locked -D warnings green before each commit)

1. **Scaffold + framework core** [5.1] — workspace member, Cargo.toml, package.json, .gitignore, lib.rs (Kind/Ctx/Outbound/Reject/Flow/Token + module decls), middleware/mod.rs (Middleware/Next/noop/spy/seq/par). Tests: seq order/short-circuit-on-reject/short-circuit-on-terminal/ctx-propagation; par concurrent/join/panic-isolation; next noop/spy.
2. **SessionRegistry + Auth** [5.2] — registry.rs, middleware/auth.rs (subtle constant-time). Tests: pass on match; reject missing/not-registered/mismatch; short-circuit without next.
3. **Observe** [5.3] — middleware/observe.rs (ObserveSink, ObservationRecord, RecordOutcome). Tests (fake sink): emit on accept/reject; latency; bound {session,correlation,component}; records auth rejection (outermost).
4. **AgentAdapter + v1 parse_hook** [4.1] — agent_adapter.rs (sync trait + ParseError + v1 impl → canonical HookEvent). Tests: one per HookKind variant (SessionStart/UserPromptSubmit/PostToolUse/PermissionRequest/Stop/SessionEnd) + invalid-json + missing-field.
5. **Normalize** [5.4] — middleware/normalize.rs. Tests: calls adapter; sets ctx.event; continues on success; rejects Invalid + skips next on parse error; agent-agnostic.
6. **Subscriptions** [5.9] — subscription.rs (per-session broadcast cap 256, ATHING_GATE_QUEUE_CAP override, drop-oldest + droppedN + log-once, subscribe/publish/end; HOOK_SUBSCRIPTION_WIRE_VERSION-gated frame codec). Tests: cap 256; drop-oldest; counter increment; no-block-on-full; deliver-to-N; per-session isolation; close-on-end; env override; wire-version carried.
7. **FanOut + PassThrough** [5.5, 5.6] — middleware/fanout.rs (publish via par, return Accepted, fire-and-forget, terminal), middleware/passthrough.rs (Forward(body) unchanged, terminal). Tests: fanout to N; returns Accepted before fanning out; non-blocking; terminal. passthrough Forward unchanged; terminal.
8. **Router + correlation** [5.7, 5.10] — router.rs (Inbound, globals [Observe,Auth], Hook=seq([Normalize,FanOut]) / Tool=seq([PassThrough]), handle()->Flow, assign-when-absent/preserve-when-supplied single point). Tests: dispatch by Kind; globals order; auth reject stops + recorded; correlation supplied/assigned/preserved/on-event/in-record.
9. **Hook ingress** [5.8] — endpoint/mod.rs (loopback + local length-prefix framing helpers), endpoint/hook.rs (axum 127.0.0.1, token from Authorization header, DefaultBodyLimit OOM guard, fire-and-forget 200 before fan-out, drives Router::handle(Hook); write gate.url). Tests: 200-before-fanout; header token; oversized-body rejected; loopback-only.
10. **Tool IPC + subscribe stream** [5.11, 5.9] — endpoint/tool.rs (length-prefixed ToolInbound over Unix socket → Router::handle(ToolCall/ToolResult) → Forward/Reject; reject malformed), endpoint/subscribe.rs (decode HookSubscribeRequest, negotiate HOOK_SUBSCRIPTION_WIRE_VERSION, stream HookEvent frames from Subscriptions::subscribe). Tests: tool inbound over IPC; Forward; reject malformed; loopback Unix; subscribe wire-version negotiation + stream.
11. **Admin + face isolation** [5.12] — endpoint/admin.rs (register/deregister on a separate authenticated loopback socket; admin token != session token; constant-time). Tests: register/deregister mutate registry; admin-token-required; constant-time; tool-route-cannot-publish-hooks; hook-cannot-register; tool-cannot-register.
12. **service-host migration** [5.13] — service.rs (Gate implements `Service`: config name="gate"; serve binds hook+tool+subscribe+admin and tracks tasks; shutdown tears down Subscriptions + SessionRegistry; health = the Probe), bin/main.rs (build v1 adapter, wire Router globals + routes, `host::run(Gate::from_env())`). Tests: serve wiring + shutdown teardown (mostly integration).
13. **Router integration tests** [5.14] — tests/router_integration.rs: handle(Hook) fans to N; handle(ToolCall/ToolResult) Forward; auth reject stops + recorded by Observe; face isolation (tool cannot publish a hook, hook cannot register); subscription teardown on session end; correlation preserved end-to-end.

## Cross-cutting risks (must hold)

- Constant-time token compare (subtle), equal-length hashes, no data-dependent branch; same primitive in Auth + admin.
- Correlation id assigned exactly once at Router entry (uuid v4 when absent), never reassigned; threaded Ctx → HookEvent.correlation_id → ObservationRecord → logs.
- Drop-OLDEST never newest; droppedN tracked from Lagged, logged once per lag; bounded O(N_sessions × 256 × event).
- Onion order load-bearing: globals [Observe outermost, Auth]; seq short-circuits and never calls next after; par isolates a panicking branch.
- Fire-and-forget: hook endpoint returns 200 before fan-out; FanOut never blocks the poster; max-body cap at transport read (before middleware).
- Loopback-only everywhere; health is the service-host Probe only (no second listener).
- R9: subscription wire = `contracts::HOOK_SUBSCRIPTION_WIRE_VERSION` (independent of the daemon `SESSION_EVENT_WIRE_VERSION`); golden cross-check parity with `@athing/sdk` (1.2).
- Dep direction (8.4): do not import daemon-pty-client; reimplement the length-prefix codec locally.
- Session-registration race (D7/R4/7.4): orchestrator registers before spawn; late hooks after deregister fail at Auth.

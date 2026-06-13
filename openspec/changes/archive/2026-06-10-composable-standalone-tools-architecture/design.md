## Context

The change establishes composable standalone tools, a shared lifecycle library, a pty-only daemon,
and a **gate** — a router that runs untrusted agent data through composable middleware over a context.
This document pins down how the gate works in practice: its context/flow types, the middleware router
(`seq`/`par`), hook and tool routing, plus the cross-tool wiring (session/token/env) and the test
strategy. It resolves the open forks surfaced during review: where normalization runs, who mints and
injects session credentials, how consumers subscribe, and what cleaning enforces on the MCP path.

In-force ADRs that constrain this design: 0001 (PTY on login), 0002 (single engine), 0003
(ports-and-adapters + composition-root DI), 0004 (hybrid adapters), 0005 (HookEvent seam), 0006
(transcript read-on-hook — **this design drops it**; content moves to the `HookEvent` payload;
flagged in Open Questions), 0007 (reliability contract), 0008 (detached daemon — **this design
amends its hook-ingress clause**; flagged in Open Questions), 0009/0010/0011 (daemon wire/fds/upgrade
— untouched here), 0012 (logging), 0013/0014/0015 (gateway — **0014 untouched**: the gateway stays a
standard MCP front and merely calls the gate).

## Goals / Non-Goals

**Goals:**

- A gate whose middleware framework (`Middleware`/`Router`/`Ctx` + `seq`/`par`) and concrete
  middleware are **modules in the gate binary**; shared wire types live in `contracts-rs`. No
  `gate-core` crate.
- A clear data-in/data-out contract: `inbound -> router(middleware over Ctx) -> outbound | rejection`.
- **Composable middleware** with `seq`/`par`. **v1 globals = `[Observe, Auth]`**; `Validate`,
  `Firewall`, `Redaction` are future middleware that drop onto a route with no gate change.
- v1 gate routes = **Hook** (Observe -> Auth -> Normalize -> FanOut) **and Tool** (Observe -> Auth ->
  PassThrough), the latter so the MCP gateway routes its tool calls through the gate.
- **Observability is a v1 feature**: every gate operation emits a correlation-bound structured record
  (ADR-0012) — the debugging surface and the substrate for future features.
- The gate normalizes raw -> canonical `HookEvent` once via an **injected adapter** (ADR-0005), so
  consumers eat the canonical event and never touch the raw agent format. The gate is agnostic in
  code (depends only on the adapter interface).
- Each middleware unit-tested in isolation; integration-tested router + service shell.

**Non-Goals (deferred past v1):**

- `validate`, `firewall`, and `redaction` middleware (the router accepts them on a route later). So in
  v1 the tool route is observe-only (no policy enforcement) and the gateway is **fail-open** on it.
- Routing PTY bytes through the gate (bytes stay direct daemon ↔ engine).
- Changing the MCP gateway's public surface (ADR-0014 stays).
- Wiring an observability exporter/collector (ADR-0012 plug points only; structured records now,
  export later).
- Daemon upgrade changes (separate change `daemon-upgrade-drain-restart`).

## Decisions

### D1. One gate binary — framework as modules; shared types in `contracts-rs`

The gate's middleware framework (`Middleware`, `Router`, `Ctx`, `Flow`, `Next`, `seq`/`par`) and its
concrete middleware (`Auth`, `Observe`, `Normalize`, `FanOut`, `PassThrough`) and the `AgentAdapter`
(`parse_hook`) live as **modules in the gate binary** — `#![forbid(unsafe_code)]`,
`#![deny(missing_docs)]`. Shared **wire types** (`HookEvent`, ids, the subscription + tool-route
message shapes) live in `contracts-rs`. There is **no `gate-core` crate**: the framework has one
consumer (the gate), and module boundaries + unit tests give the same testability a crate would.

Memory does **not** link the gate's framework — it **subscribes** to the gate for canonical events.
(Option 1: the gate is the universal agent-hook ingress; a memory-only deployment is memory + gate.)

_Alternative rejected:_ a `gate-core` crate — one consumer doesn't justify a separate compilation
unit; `contracts-rs` already carries the only types other crates share.

### D2. Context + flow types (the contract)

```rust
// gate binary — internal framework types (wire types like HookEvent live in contracts-rs)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Hook, ToolCall, ToolResult }   // all v1; the router dispatches on this

/// Threaded through the middleware; enriched as it goes (e.g. Normalize sets `event`).
pub struct Ctx {
    pub kind: Kind,
    pub session: SessionId,
    pub correlation: CorrelationId,
    pub token: Token,
    pub body: Bytes,                 // inbound payload
    pub event: Option<HookEvent>,    // set by Normalize on the hook route; read by FanOut
}

/// Outcome of handling an inbound. (thiserror Reject — ADR-0007 typed taxonomy.)
pub enum Outbound { Accepted, Forward(Bytes) }  // Hook -> Accepted (fanned out) ; Tool -> Forward(body)

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Reject {
    #[error("unauthenticated")] Unauthenticated,
    #[error("invalid: {0}")]    Invalid(String),
    #[error("denied: {0}")]     Denied(String),
}

/// A middleware returns an outcome or a typed rejection.
pub type Flow = Result<Outbound, Reject>;

/// The continuation — run the rest of the chain. Single-use (consumes self).
pub struct Next<'a> { /* remaining middleware + terminal */ }
impl<'a> Next<'a> { pub async fn run(self, ctx: Ctx) -> Flow; }
```

Onion model: a middleware **continues** by calling `next.run(ctx)` and **short-circuits** by
returning `Ok(Outbound)`/`Err(Reject)` without calling `next`. This is what lets `Observe` wrap the
chain and see the outcome.

### D3. The gate is a router over composable `Middleware`

A middleware is an onion unit over `Ctx`: do work, then call `next` (or short-circuit). The router
dispatches an inbound to global middleware + the route for its `Kind`. `seq`/`par` combinators let
the composition root declare what runs sequentially vs concurrently.

```rust
#[async_trait]
pub trait Middleware: Send + Sync {
    async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow;   // call next.run(ctx), or return Ok(Outbound)/Err(Reject)
}

pub fn seq(items: Vec<Arc<dyn Middleware>>) -> Arc<dyn Middleware>;  // sequential; first Reject/Done stops
pub fn par(items: Vec<Arc<dyn Middleware>>) -> Arc<dyn Middleware>;  // concurrent, joined (side-effects / independent checks)
```

The composition root wires the router. **Order is onion: the first `.global()` is the outermost
layer.** `Observe` goes first so it wraps `Auth` and records auth _rejections_ too. **v1 globals =
`[Observe, Auth]`**; route handlers per kind; future middleware drop in with no gate change:

```rust
let gate = Gate::router()
    .global(Observe)                               // OUTERMOST "around": start -> next -> emit record (D12)
    .global(Auth)                                  // inner; may Reject (per-session token) — Observe still records it
    .on(Kind::Hook,       seq([Normalize(adapter), FanOut]))   // FanOut publishes to N consumers via par
    .on(Kind::ToolCall,   seq([PassThrough]))      // v1 nothing; Validate/Firewall slot here later
    .on(Kind::ToolResult, seq([PassThrough]));

// entry — route an inbound
let flow: Flow = gate.handle(inbound).await;       // Ok(Accepted) for hooks ; Ok(Forward(body)) | Err(Reject) for tools
```

A **max-body-size cap lives in the transport read** (always-on OOM guard), independent of middleware.

`Arc<dyn Middleware>` + `Send + Sync` so the router shares middleware across async tasks. Each
middleware is a small unit, tested in isolation with a fake `next` (Auth pure -> trivial; Observe /
FanOut with fakes). Adding policy = add a middleware to a route; the gate is untouched.

_Alternative rejected:_ a hard-coded `auth(); validate(); firewall();` sequence — not extensible, and
adding redaction later would touch every call site.

#### Authoring a middleware — the four shapes

Every middleware is one of four shapes. Pick by what it does to the flow:

```rust
// 1. GATE — pass or reject (Auth, RateLimit, Firewall)
async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
    if self.allow(&ctx) { next.run(ctx).await } else { Err(Reject::Denied("...".into())) }
}

// 2. AROUND — observe/time; capture what you need BEFORE moving ctx into next (Observe)
async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
    let (s, c, k) = (ctx.session, ctx.correlation, ctx.kind);
    let started = self.clock.now();
    let flow = next.run(ctx).await;                  // run the rest, then see the outcome
    self.sink.record(s, c, k, &flow, self.clock.since(started));
    flow
}

// 3. TRANSFORM — enrich ctx, continue (Normalize)
async fn handle(&self, mut ctx: Ctx, next: Next<'_>) -> Flow {
    ctx.event = Some(self.adapter.parse_hook(&ctx.body).map_err(|e| Reject::Invalid(e.to_string()))?);
    next.run(ctx).await
}

// 4. TERMINAL — produce the outbound, ignore next (FanOut, PassThrough)
async fn handle(&self, ctx: Ctx, _next: Next<'_>) -> Flow {
    Ok(Outbound::Forward(ctx.body))
}
```

Test each in isolation with a fake `next` (`Next::noop()` returns `Ok(Accepted)`; `Next::spy()` records
whether `run` was called). Pure middleware (Auth, RateLimit) test trivially; impure ones (Observe,
FanOut) take a fake sink/subscribers. Wire a new one at the composition root — one line, gate untouched:
`.on(Kind::ToolCall, seq([RateLimit::new(60), PassThrough]))`.

Gotchas: `ctx` **moves** into `next.run(ctx)` — copy `session`/`correlation` first if you need them
after; `Next` is **single-use** (one `run`); state lives in the middleware (`&self`), built at the root.

### D4. The gate normalizes raw -> canonical `HookEvent` once, via an injected adapter (resolves gap B + the drift risk)

The gate normalizes **once**: raw agent hook -> canonical `HookEvent` (the `contracts-rs` /
`@athing/sdk` type), via an injected agent adapter — ADR-0005 (the ingress calls `parse_hook` and
routes by session) + ADR-0003 DI. Both consumers eat the canonical event:

- the engine uses `HookEvent.type` for status and the typed payload for content (no transcript);
- memory maps the canonical event to capture chunks (`UserPromptSubmit`->prompt, `PostToolUse`->tool),
  with no raw-format knowledge.

On the `Hook` route this is two middleware after the globals: `Normalize` (calls the injected
adapter) then `FanOut` (publishes to subscribers):

```rust
// Normalize middleware — agnostic in code; concrete adapter injected at the root
async fn handle(&self, mut ctx: Ctx, next: Next<'_>) -> Flow {
    match self.adapter.parse_hook(&ctx.body) {        // injected Rust adapter
        Ok(event) => { ctx.event = Some(event.with_correlation(ctx.correlation)); next.run(ctx).await }
        Err(e)    => Err(Reject::Invalid(e.to_string())),
    }
}
// FanOut middleware — terminal; publishes to all subscribers of the session
async fn handle(&self, ctx: Ctx, _next: Next<'_>) -> Flow {
    let ev = ctx.event.ok_or(Reject::Invalid("not normalized".into()))?;
    self.subs.publish(ctx.session, ev).await;         // par to N consumers
    Ok(Outbound::Accepted)
}
```

Consequences:

- **One** raw parser -> zero raw-format drift. memorya's stray vocabulary becomes a trivial map from
  the canonical event, and the previously-undefined raw->curated mapper is eliminated.
- `parse_hook` for hooks now lives in **Rust**, behind an injected `AgentAdapter` trait — a **module
  in the gate binary** in v1 (no separate crate; promote to a per-agent adapter crate at multi-agent).
  The TS `parseHook` for hooks **retires** (the engine consumes the canonical event from the gate).
- The adapter is now **single-language (Rust)**: `parseHook` is its only parse function. Transcript
  read-on-hook (ADR-0006) is **dropped** with this change, so `parseTranscriptEntry`/`transcriptPath`
  are removed — there is no TS-side adapter parse left, and the bi-lingual split disappears.
- **Canonical payload must be rich enough for both status and content/capture**: `HookEvent`'s payload
  carries typed fields (prompt content; tool name/input/response; turn index) so both the engine
  (content) and memory (capture) read it directly; no consumer needs the raw format. Usage/cost that
  only the transcript carried is dropped with the feature (accepted).

_Alternative rejected:_ gate forwards raw and consumers parse — creates a second Rust raw-parser and
cross-language drift; memory has no raw parser today.

### D5. Tool inbounds are just another route (resolves gap E)

The tool gateway sends a `ToolCall`/`ToolResult` **inbound** to `gate.handle()`; the router runs the
globals (`Observe`, `Auth`) then the tool route. In v1 the tool route is `[PassThrough]` — authenticate

- observe + return the body unchanged. Its v1 value is **observability**: every tool call/result flows
  through the gate -> one correlation-tagged view of all agent traffic (D12). Policy (`Validate`/`Firewall`)
  is added to this route later.

```rust
// tool gateway side — send an inbound, get an outbound
let outbound = gate.handle(Inbound { kind: Kind::ToolCall, session, correlation, token, body }).await;
match outbound {
    Outbound::Forward(bytes) => /* send to backend */,
    // Reject only once Firewall/Validate exist; v1 PassThrough never rejects
}
```

The gateway talks to the gate over local IPC (reusing the length-prefixed loopback framing); the gate
never speaks MCP. One router = one observability point now and one policy point later.

**Fail-open in v1, fail-closed later.** With observe-only and no policy, if the gate is unreachable
the gateway SHALL log a warning and proceed (losing only the observability tap, not security). Once
`Validate`/`Firewall` land on the tool route, this flips to **fail-closed** (gate down -> reject).

**Policy source (future, with `firewall`).** Allow/deny rules and size limits will live in a config
file owned by the orchestrator, loaded at session registration; default posture **default-allow +
deny-list** for the local single user, allowlist by configuration. Not part of v1.

### D6. Routing: a per-session pub/sub router (resolves gap D)

```rust
pub struct Router { /* session_id -> bounded broadcast sender */ }
impl Router {
    pub fn subscribe(&self, s: SessionId) -> Subscription;  // consumer stream
    pub fn publish(&self, s: SessionId, ev: HookEvent);     // fan out to subscribers
    pub fn end(&self, s: SessionId);                        // drop on session end
}
```

Channels are **bounded** (ADR-0007). The poster is a fire-and-forget bash hook that cannot be
backpressured, so on a full channel the gate SHALL **drop the oldest event, increment a dropped
counter, and log** (never grow unbounded, never block). Dropping is safe because authoritative
terminal state (session exit) reaches the engine over the **daemon session-event stream**, not the
hook path — so a dropped hook only loses self-correcting intermediate status, never a terminal
transition. The hook-subscription wire is versioned and mirrored in `contracts-rs` +
`@athing/sdk` (TS) so memory (Rust) and the engine (TS) observe the same shape.

### D7. Session credentials and env injection (resolves gap C)

The daemon stays oblivious; the orchestrator owns wiring:

1. Orchestrator mints `session_id` + per-session `token`.
2. Orchestrator registers `(session_id, token, allow-policy)` with the gate over the **admin**
   surface — **before** spawning the agent (avoids a race where a hook arrives unregistered).
3. Orchestrator hands the daemon a spawn env containing the gate hook URL + `session_id` + `token`.
4. Daemon spawns the agent with that env (pass-through; it never learns what the env is for).
5. The agent's installed hook posts to the gate URL with the token; the `Auth` stage verifies it.

This keeps ADR-0008's "daemon" while moving hook ingress to the gate, and preserves daemon
obliviousness.

**Teardown.** The authoritative end-of-session signal is the **daemon session-exit event** (the
orchestrator already observes the PTY session subscription), not the best-effort `SessionEnd` hook
(the agent may crash without firing it). On observing session exit the orchestrator deregisters the
session from the gate, which drops its token and ends its subscriptions. A late hook arriving after
deregistration fails the `Auth` stage (token gone) and is rejected — correct by construction.

### D8. Consumer wiring (resolves gap A)

- **Two wires, a Rust + a TS client each (symmetric):** the **PTY wire** -> `daemon-pty-client` (Rust,
  used by the desktop's Tauri/Rust side) and `proxy.ts` (TS, used by the engine); the **gate hook
  subscription** -> `gate-client` (Rust, used by memory) and a TS gate client in `apps/server` (feeding
  the engine). Both Rust clients are thin libs that decode the versioned wire from `contracts-rs`; no
  shared client crate, no tool->tool dependency (memory depends on `gate-client`, never the `gate` crate).
- The engine's gate subscription is hosted by the **server** (the TS gate client); the engine stays
  transport-agnostic per ADR-0005 (it consumes `HookEvent`, not a transport). The engine's TS
  `parseHook` for hooks retires — the gate already normalized. Memory does **not** depend on the daemon.
- The MCP gateway sends tool inbounds (`ToolCall`/`ToolResult`) to `gate.handle()` and uses the outbound.
- Consumers eat the **canonical `HookEvent`** the gate emits: the engine reads `type` for status,
  memory maps the event to chunks. Neither touches the raw agent format; the gate's injected adapter
  is the single place that does.

### D9. Correlation id threads every message (extends ADR-0012 bound context)

`session_id` correlates by session but cannot distinguish concurrent actions within one session. A
`correlation_id` identifies a single logical action and travels with it across process hops, so one
hook can be followed `ingress -> auth -> normalize -> fan out -> capture`, and one tool call
`gateway -> gate.handle(ToolCall) -> backend -> result`.

Rules:

- `correlation_id` is a field on `Ctx` and on all three new wire contracts — the hook envelope, the
  hook-subscription frames, and the tool inbound/outbound — mirrored in `contracts-rs` + `@athing/sdk`.
- The gate assigns a fresh `correlation_id` for a hook if the caller did not supply one; the gateway
  supplies one on a tool inbound and reuses it for its own backend call so the two sides line up.
- It is shaped as an opaque string so it can later carry a W3C `traceparent` without a wire change.
- Every gate log line binds `{ session_id, correlation_id }` as context (ADR-0012 bound-context),
  and each emitted `HookEvent` carries the `correlation_id` so downstream logs (memory, engine) bind
  the same value.

This is bound context that happens to cross a process — **not** distributed tracing. Spans,
exporters, and trace-context _propagation machinery_ stay deferred per ADR-0012; only the id field
and log binding are added now (cheap on a fresh wire, costly to retrofit later).

_Alternative rejected:_ rely on `session_id` alone — cannot disambiguate concurrent events, and
adding an id to the wire after the fact is a breaking change.

### D10. No compression on internal IPC (decided non-goal)

All internal hops — daemon ↔ engine bytes, gate hook subscription, gate tool route — are loopback unix
sockets on one host. Localhost throughput is effectively memory speed, so compression would spend CPU
and add latency to save bandwidth that is not scarce; it also fights the raw-bytes-end-to-end value
(ADR-0001/0009) and the existing flow-control that already bounds volume (ADR-0010). Therefore
internal services SHALL NOT compress.

Compression belongs only at a genuine **network** edge — the server's WebSocket to a remote browser
UI — where it is handled by the transport's standard mechanism (`permessage-deflate`), not by a
bespoke codec. That edge is outside this change.

_Alternative rejected:_ compress the daemon byte stream or the gate wires — net-negative on localhost
(CPU > saved bandwidth) and complicates the raw-byte contract for no local benefit.

### D11. Canonical `HookEvent` payload shape

The canonical event is the single contract the gate's adapter produces and both consumers read. It
enriches ADR-0005's `{ sessionId, type, payload? }` with a correlation id, a timestamp, and a
**typed per-type payload** carrying everything memory's capture needs — so no consumer touches the
raw agent format. Defined once in `contracts-rs` and mirrored in `@athing/sdk` (TS); the
wire is camelCase JSON.

```rust
// contracts-rs
pub struct HookEvent {
    pub session_id: SessionId,
    pub correlation_id: CorrelationId,
    pub ts: i64,                 // epoch millis; adapter fills from raw when present, else gate stamps
    #[serde(flatten)]
    pub kind: HookKind,          // discriminant + its typed payload
}

#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum HookKind {
    SessionStart     { cwd: Option<String>, client: Option<String>, cli_version: Option<String> },
    UserPromptSubmit { content: String, turn_index: Option<i64> },
    PostToolUse      { tool_name: String, tool_input: serde_json::Value, tool_response: String, turn_index: i64 },
    PermissionRequest{ tool_name: Option<String>, request: serde_json::Value },
    Stop             { turn_index: Option<i64> },
    SessionEnd       { reason: Option<String> },
}
```

```ts
// @athing/sdk (TS) — mirror; field names match the camelCase wire
export type HookEvent = { sessionId: string; correlationId: string; ts: number } & HookKind;
export type HookKind =
  | { type: "SessionStart"; payload: { cwd?: string; client?: string; cliVersion?: string } }
  | { type: "UserPromptSubmit"; payload: { content: string; turnIndex?: number } }
  | {
      type: "PostToolUse";
      payload: { toolName: string; toolInput: unknown; toolResponse: string; turnIndex: number };
    }
  | { type: "PermissionRequest"; payload: { toolName?: string; request: unknown } }
  | { type: "Stop"; payload: { turnIndex?: number } }
  | { type: "SessionEnd"; payload: { reason?: string } };
```

Consumer mapping (proves the shape is sufficient):

| `type`            | engine -> status | memory -> capture                                             |
| ----------------- | --------------- | ------------------------------------------------------------ |
| SessionStart      | IDLE            | `ensure_session(cwd, client)`                                |
| UserPromptSubmit  | WORKING         | `capture_prompt(content, turnIndex)`                         |
| PostToolUse       | WORKING         | `capture_tool(toolName, toolInput, toolResponse, turnIndex)` |
| PermissionRequest | WAITING_INPUT   | skip (low-value per `memorya-capture` skip-list)              |
| Stop              | IDLE            | — (may trigger consolidation)                                |
| SessionEnd        | DONE            | — (deregister; may trigger consolidation)                    |

Notes:

- The engine reads `type` for status and the typed payload for content; there is no transcript read
  (ADR-0006 dropped). The typed payload serves both the engine (content) and memory (capture).
- The adapter is **clock-free** (pure): it fills `ts` only if the raw hook carries one; otherwise the
  gate's I/O shell stamps it at receive time. Keeps `parse_hook` deterministic and unit-testable.
- `toolResponse` is text; if the raw response is structured the adapter stringifies it (memory stores
  text). `toolInput` stays structured (`Value` / `unknown`).
- A new agent event type is an **additive** enum variant + a parser unit test; consumers that do
  not handle it ignore it (engine falls through to its default status; memory skips).

### D12. Observability is a v1 gate feature

The gate is the single chokepoint for all agent-facing traffic (every inbound the router handles),
which makes it the natural observability seam — the reason the gateway routes through it even before
policy middleware exist. Observability is the **`Observe` global middleware** (an "around" unit:
stamp start -> `next` -> emit the record with outcome + latency):

```
{ ts, sessionId, correlationId, kind: Hook|ToolCall|ToolResult,
  eventType?, outcome: accepted|rejected(reason), fanoutN?|backend?, latencyMs, droppedN? }
```

- **Bound context (ADR-0012):** a child logger bound once with `{ sessionId, correlationId,
component: "gate" }`; every record inherits it. The gate is constructed with a Resource identity
  (service name/version, pid) — exactly ADR-0012's plug points. No exporter is wired (deferred);
  only the structured shape and binding are added now.
- **It is middleware, not special-cased** — `Observe` is wired as a global, so it runs for every
  inbound the router dispatches. (The emit itself is I/O; the middleware that do I/O are the impure
  ones, tested with a fake sink.)
- **Why now:** it is the debugging surface (one correlation-tagged view across processes via D9) and
  the substrate for future features (metrics, audit, tracing spans via `tracing-opentelemetry` later).
  The correlation id (D9) is what makes records joinable across the daemon/gate/gateway/memory hops.

_Alternative rejected:_ defer observability with the policy stages — but then v1 routing through the
gate buys nothing, and the cheapest, highest-leverage feature (visibility) is the one most useful
during the build-out.

## Risks / Trade-offs

- [The tool route's IPC hop per tool call/result adds latency] -> it is Auth + Observe + PassThrough
  in v1; local unix-socket IPC is sub-millisecond and tool calls are already slow; keep middleware
  O(n) over bytes.
- [Dropping transcript content (ADR-0006) loses usage/cost and any detail the hook payload omits] ->
  accepted: content is whatever the `HookEvent` payload carries; the UI still shows live activity via
  the raw PTY stream. If richer content is wanted later, it is a new, separate source — not a revival
  of transcript coupling.
- [The canonical `HookEvent` payload must be rich enough for both status and content/capture] -> define
  its typed payload (prompt content; tool name/input/response; turn index) up front; parser unit tests
  assert the gate's adapter fills it. The adapter is single-language (Rust `parseHook` only), so there
  is no cross-language parser to keep in sync.
- [Registration race: a hook arrives before the session is registered] -> the orchestrator registers
  with the gate before requesting the spawn (D7 step 2 precedes step 3).
- [A slow subscriber could stall fan-out] -> bounded channels with a logged drop policy (ADR-0007); one
  slow consumer never blocks others.
- [Constant-time token compare is easy to get wrong] -> use a vetted constant-time comparison; unit-test
  it; the daemon already has a reference implementation to mirror.

## Migration Plan

1. Gate framework modules in the gate binary (`Middleware`/`Router`/`Ctx`/`Flow`/`Next` + `seq`/`par`);
   unit-tested with fake `next`.
2. Define the canonical `HookEvent` (typed payload) in `contracts-rs` + `@athing/sdk`; implement the
   `AgentAdapter` (`parse_hook`) as a module in the gate binary (no separate crate yet); retire the TS
   `parseHook` for hooks.
3. Gate binary: `Auth`/`Observe`/`Normalize`/`FanOut`/`PassThrough` middleware, router wiring,
   endpoints; integration tests.
4. Tool route (`PassThrough`) + `Observe` (D12); the MCP gateway sends tool inbounds to `gate.handle()`
   (fail-open) and adopts `service-host` (otherwise unchanged).
5. Memory subscribes to the gate (capture, mapping canonical `HookEvent` -> chunks); engine subscribes
   to the gate (status/content).
6. Orchestrator: session registration + env injection (D7).
7. Remove hook ingress from the daemon (daemon becomes pty-only).
8. Rollback: re-point the installed hook at the daemon's old ingress and restore the daemon relay;
   each step is independently revertible pre-v1.
9. Future (not v1): add `Validate`/`Firewall`/`Redact` stages — `clean()` gains teeth and the gateway
   flips to fail-closed; wire an observability exporter.

## Testing Strategy (per-middleware units + router integration)

**Middleware (each in isolation, with a fake `next`):**

- `Auth` (pure) — table-driven, descriptive names, one assertion per test:
  `auth_rejects_when_token_mismatch`, `auth_passes_when_token_matches`.
- `Normalize` — delegates to the adapter; assert it calls `next` with the event set, rejects on parse error.
- `Observe`/`FanOut` (impure) — with a fake sink / fake router: assert a record is emitted, assert
  publish-to-N.
- Combinators: `seq` short-circuits on the first `Reject`/`Done` (a recording middleware observes no
  later call); `par` runs all and joins.

**Adapter (`parse_hook`, Rust) — parser unit tests:** table-driven cases mapping a synthetic raw hook
-> expected canonical `HookEvent`, one per event type. When the agent's format changes, update the
expected case deliberately. (No transcript parser to test — that feature is dropped.)

**Router integration (stub adapter + stub consumers + in-memory router):**

- `handle(Hook)` -> Observe -> Auth -> Normalize -> FanOut to N subscribers.
- `handle(ToolCall)` -> Observe -> Auth -> PassThrough -> `Outbound::Forward(bytes)`; and the auth-reject path.
- Auth rejection on a bad/missing token.
- **Face isolation:** a tool-route caller cannot cause a hook event to be published (router untouched).
- Subscription teardown on session end.
- **Correlation id preserved:** a supplied `correlation_id` appears on the fanned-out `HookEvent`;
  an absent one is assigned and is stable across Auth -> Normalize -> FanOut.

**Conventions (Apollo Rust best practices):** typed `Reject` via `thiserror`; no `unwrap`/`expect`
outside tests; borrow over clone where possible; small `Middleware` trait; `Arc<dyn Middleware>` only
at the router boundary; `Send + Sync` shared middleware; `#![forbid(unsafe_code)]` +
`#![deny(missing_docs)]` on libraries; CI runs `cargo clippy --all-targets --all-features --locked -- -D warnings`.

## Open Questions

Recorded by the **adr step**:

- **ADR-0016** amends the hook-ingress clause of ADR-0008: hook ingress moves out of the daemon to
  the gate (daemon becomes pty-only) and records the composable-tools topology + the gate.
- **ADR-0017** supersedes ADR-0006: structured content now comes from the canonical `HookEvent`
  payload, not the transcript; `parseTranscriptEntry`/`transcriptPath` are removed. The adapter is
  **single-language (Rust `parseHook` only)** — ADR-0004 stays satisfied (one adapter, one language)
  and ADR-0005 is **honored** (the gate, as the ingress, calls `parseHook`). ADR-0006 is flipped to
  `superseded by ADR-0017`; ADR-0008's status is annotated as amended by ADR-0016.

## Resolved decisions (build-plan gate)

Settled before implementation; bake the concrete values into the tasks/specs noted.

- **R1 — Canonical `HookEvent` payload.** Freeze D11 as the contract verbatim: `HookEvent { sessionId, correlationId, ts (epoch millis), kind }` with `kind` the tagged enum and its typed per-type fields. `ts` is adapter-filled from raw when present, else gate-stamped. camelCase wire. (tasks 1.1, 1.2, 4.1, 6.4)
- **R2 — Per-session delivery.** Bounded broadcast, capacity **256 events**, drop-**oldest**, increment `droppedN` and log once per lag. Override via `ATHING_GATE_QUEUE_CAP`. (task 5.9)
- **R3 — adopt-or-spawn version match.** Exact string match on the manifest `version`; mismatch -> respawn. No semver. The agent CLI's `cliVersionRange` is a separate, unrelated concern. (task 2.5)
- **R4 — Gate session/policy store.** In-memory registry `sessionId -> {tokenHash, allowPolicy}`, populated via the admin register/deregister surface; no on-disk policy file in v1. Caveat: a gate restart drops sessions (re-registered by the orchestrator, or they die with the agent; durable persistence belongs to `daemon-upgrade-drain-restart`). (tasks 5.2, 5.12, 7.4)
- **R5 — Memory dual-mode detection.** Face by subcommand: `memorya mcp` = standalone MCP stdio + viewer; `memorya serve` = viewer only. Capture source by env: `ATHING_GATE_URL` present -> subscribe to the gate (composed); absent -> standalone (stub/none). (task 6.7)
- **R6 — `spawn_fields_differ` set.** Spawn-affecting = `{command, args, cwd, env[allowlist]}` with a per-tool declared env-key allowlist (e.g. `ATHING_DIR`, gate URL/session/token). Non-affecting (ignored) = logging level, observer/metadata, manifest-only fields. (task 2.6)
- **R7 — `ATHING_DIR` resolution.** Parity with the existing TS + Rust behavior: if set, resolve against cwd (absolute passes through); else default `~/.athing`. service-host and process-launch mirror it exactly. (tasks 2.1, 2.5)
- **R8 — Tool-gateway fail-open.** The tool route is fail-open in v1 (gate unreachable -> log + forward); flips to fail-closed once policy middleware lands. The hook route is separate (fire-and-forget; gate-down means hooks are lost). (task 7.1)
- **R9 — Subscription wire versioning.** The daemon session-event wire and the gate hook-subscription wire are independently versioned: each carries its own `WIRE_VERSION` const in `contracts-rs`, negotiated per-connection via the existing hello/ack capability mechanism. (tasks 1.1, 3.2, 5.9)

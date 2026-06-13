## Why

The Rust side grew three long-running tools (PTY daemon, memory layer, MCP gateway) plus two
orchestrators (server, desktop). Each tool independently reimplements the same "local service"
machinery — directory resolution, manifest read/write, socket binding, signal handling,
adopt-or-spawn, child supervision. The result:
the same lifecycle code is copy-pasted across crates, and the dependency direction is muddy.
Worse, there are two separate hook ingresses (the daemon's and the memory layer's), the memory
layer runs its own MCP front bypassing the gateway, and untrusted agent input is validated
ad hoc in several places with no single trust boundary.

The separation of tools is a feature, not the problem — users should be able to run only memory,
only the tool gateway, or only the PTY daemon, and compose them when they want more. The fixes
this change makes: (1) a shared lifecycle library so every tool gets "run-me" plumbing for free;
(2) a **gate** — an agent-facing router over composable middleware (**v1 globals: `Observe` (outermost),
`Auth`**; validate, firewall, redaction are future middleware) with observability on every
inbound; it hosts the agent-hook route natively (the installed hook posts to it) and a tool route
other ingestors send to; (3) the PTY daemon shrinks to
**pty-only** — it no longer ingests hooks; (4) memory becomes a library with two well-placed faces;
(5) thin optional orchestrators that bootstrap and wire tools without the tools depending on them.

The gate gives the system one chokepoint for all agent-facing traffic: in v1 it authenticates and
**observes** everything (the debugging/feature substrate), and gains policy (validate/firewall/
redaction) later with no caller change. The gate never speaks tool protocols — the tool gateway owns
MCP and routes its tool calls through the gate; the PTY daemon owns the file descriptors and never sees
untrusted ingress.

## What Changes

- **Split lifecycle into two libraries.** A `service-host` library owns "run-me" concerns
  (dir/manifest/socket/signals/shutdown/liveness probe) used by every long-lived tool. A
  `process-launch` library owns "run-others" concerns (spawn, adopt-or-spawn, restart,
  spawn-field diffing) used only by orchestrators and by the gate over its external backends.
- **Introduce the gate. BREAKING.** A **router** that runs every inbound through composable
  **middleware** over a context (`seq`/`par`, declared at the composition root). **v1 globals =
  `[Observe, Auth]`**; `validate`, `firewall`, `redaction` are future middleware that drop onto a
  route with no gate change. The `Middleware`/`Router`/`Ctx` contracts live in a library; the gate
  process is a thin shell holding the concrete middleware + transport. Two routes in v1: the **Hook
  route** (the installed agent hook posts, fire-and-forget; Observe -> Auth -> Normalize via injected
  adapter -> FanOut by session id) and the **Tool route** (other ingestors send tool inbounds). The
  gate never speaks a tool protocol and holds no backend registry. Routes/surfaces are isolated
  (hook != tool != admin).
- **The gate is the observability chokepoint — v1 feature.** `Observe` is a global middleware: every
  inbound emits a structured, correlation-bound record (ADR-0012: `{ sessionId, correlationId }`
  bound context + a gate Resource identity). This is the debugging substrate and the foundation for
  future features (metrics, audit, tracing).
- **The tool gateway uses the gate in v1 — for observability.** The MCP gateway sends `ToolCall`/
  `ToolResult` inbounds to `gate.handle()`. In v1 the Tool route is `Observe -> Auth -> PassThrough`
  (policy middleware deferred), so its value now is a single, correlation-tagged view of all agent
  tool traffic. It is **fail-open** in v1 (gate down -> gateway logs a warning and proceeds; no policy
  to enforce yet); it flips to **fail-closed** once `validate`/`firewall` land on the route. The
  gateway is otherwise unchanged and adopts `service-host`. ADR-0014 stays intact.
- **Drop transcript read-on-hook. BREAKING (supersedes ADR-0006).** Structured content now comes from
  the canonical `HookEvent` payload (which already carries tool name/input/response for memory), not
  from reading the on-disk transcript. `parseTranscriptEntry` and `transcriptPath` are removed, which
  leaves the agent adapter **single-language** (Rust `parseHook` only). Usage/cost that only the
  transcript carried is dropped with the feature.
- **PTY daemon becomes pty-only. BREAKING.** Hook ingress and hook fan-out (`relay_hook`,
  `hooks.sock`) move out of the daemon to the gate. The daemon keeps only PTY ownership: fds,
  raw byte streaming, session control (spawn/kill/resize), and a session-event subscription
  (lifecycle: start/exit/status). It carries no knowledge of downstream consumers.
- **PTY bytes never flow through the gate.** The hot path stays direct: daemon ↔ engine/UI over
  the existing binary framing. The gate handles hook and tool inbounds only — never the byte
  stream — so it adds zero latency to interactive rendering.
- **Rule: an athing tool is launched only by an orchestrator; no athing tool spawns another.**
  Tools connect through contracts, never by spawning each other. The tool gateway is the one
  process allowed to spawn children, and only _external_ (non-athing) MCP backends — never an
  athing tool. When memory is used as a backend, the orchestrator launches memory and the gateway
  connects to the already-running instance. This removes the adopt-or-spawn logic smeared across
  crates.
- **Three contract surfaces, pinned to one spec. BREAKING.** (1) the **PTY session subscription**
  (daemon -> consumers: bytes + lifecycle); (2) the **gate hook subscription** (gate -> consumers:
  canonical hook events by session id); (3) the gate **tool route** (tool inbound/outbound). All are mirrored in
  `contracts-rs` and `@athing/sdk` (TypeScript), pinned to one versioned spec so they cannot drift.
  Each wire gets a thin Rust client + a TS client: the **PTY wire** -> `daemon-pty-client` (Rust, used
  by the desktop) and `proxy.ts` (TS, used by the engine); the **gate hook subscription** ->
  `gate-client` (Rust, used by memory) and a TS gate client in the server (feeding the engine). Both
  Rust clients decode the versioned wire from `contracts-rs`. Memory does NOT depend on the daemon; no
  client is ever depended on by the daemon. All three wires carry a `correlation_id` alongside
  `session_id` so one action can be followed across process hops.
- **Memory subscribes to the gate. BREAKING.** The memory tool no longer runs its own HTTP hook
  ingress. It consumes canonical `HookEvent`s through a `HookSource` port whose v1 adapter is the
  **gate subscription** (the port stays so capture is testable with a stub source). The gate is the
  universal ingress, so a memory-only deployment is **memory + gate** (no daemon, no MCP gateway).
- **Memory recall is dual-mode.** Standalone, memory exposes recall as its own standard MCP front.
  Composed, the orchestrator launches memory and the tool gateway fronts its recall as an ordinary
  backend (no special-casing), preserving "one MCP front." The human-facing recall viewer on
  loopback (required by the `memorya-recall` spec) is retained in both modes.
- **Capture queue is made durable and proactively drained.** The `memorya-capture` spec already
  mandates fire-and-forget hooks and out-of-band ("enqueued for asynchronous processing")
  embedding, so async capture is not new. This refines it: the enqueue becomes a durable row that
  survives restart, and the lazy `embed_pending` flush becomes a proactive background-worker drain
  (embedding on a blocking pool) so recall no longer pays the backlog. The queue lives inside the
  memory tool. A strengthening of an existing requirement, not a contract break.
- **Memory stops being an "app" with its own process-management surface. BREAKING.** Storage,
  recall, embeddings, chunking, consolidation stay in `memorya-rs`'s existing library (it is already
  lib + bin); the memory tool is the thin bin over it, and ad-hoc process glue moves to
  `service-host`. No new `memorya-core` crate. The loopback human viewer is retained (required by
  `memorya-recall`); only the hook-ingress path and process plumbing change.
- **Orchestrators become thin and optional.** Server and desktop use `process-launch` to bootstrap
  whichever tools the user selected and wire the ports. Every tool still runs without any
  orchestrator. The cli's hook installer points the agent's installed hook at the gate (the universal
  ingress); memory subscribes for events.

## Before / After Architecture

### Before

```
agent settings.json -- 1 hook --> agent CLI (PTY)
                                       |
                  +--------------------+--------------------+
                  v (path A)                                v (path B, parallel)
            hooks.sock (daemon)                       memory POST /hook
            token auth, fan-out                       own ingress, own token
                  |                                          |
   +--------------v-------------+            +---------------v------------+
   | daemon  [PROCESS 1]        |            | memory  [PROCESS 2]        |
   | PTY fds, daemon.sock       |            | capture/recall/embed       |
   | relay_hook (hook ingress)  |            | own SQLite, own MCP stdio  |--+ MCP bypasses
   | manifest + athing_dir  (D) |            | manifest + athing_dir  (D) |  | the gateway
   | adopt-or-spawn         (D) |            | adopt-or-spawn         (D) |  |
   +--------------+-------------+            +----------------------------+  |
                  | binary framing                                          |
                  v                                                         |
        @athing/engine (thin TS client) --> apps/server --> apps/ui        |
                                                                            |
   +----------------------------+         +-------------------------------v-+
   | gateway [PROCESS 3]        |         | gateway aggregates external      |
   | single MCP front           |         | backends, but NOT memory          |
   | supervisor spawn-diff  (D) |         +-----------------------------------+
   | manifest + athing_dir  (D) |
   +----------------------------+         (D) = duplicated service-host pattern
   desktop: adopt-or-spawn (D)                copy-pasted across crates
```

Problems: two hook ingresses, two MCP fronts, no single trust boundary for untrusted agent input,
daemon doing both PTY framing and hook ingress, and the "(D)" lifecycle pattern reimplemented
across daemon, gateway, and desktop.

### After (gate = middleware router; tool gateway stays plain)

```
   CONTRACTS (pure, shared, zero I/O):  HookEvent . SessionEvent . MCP types . wire codec

   daemon (pty-only)            GATE (router + middleware)      tool gateway (MCP)
   PTY fds, raw bytes,          globals: [Observe, Auth]        standard MCP front (0014)
   session control,             Hook route:  Normalize->FanOut   aggregates + supervises
   session-event stream         Tool route:  PassThrough        sends ToolCall/ToolResult
   NO hook ingress              hook endpoint <--post-- agent     inbounds to gate.handle()
        |                         |  bash hook                     |
        | bytes (hot path,        |  fan out HookEvent by session  | observe pass-through
        | DIRECT, never           v                                | (fail-open in v1)
        | through gate)     memory (capture)   engine/UI <---------+ memory recall
        v                        ^                  ^                = a backend here
   engine/UI <-------------------+------------------+
   (session events via daemon-pty-client; canonical HookEvents from the gate)

        +------ all long-lived tools use service-host (run-me) ------+
   ---------------------------------------------------------------------
   THIN ORCHESTRATORS (optional; degrade if a tool is absent):
   server . desktop . cli  --> process-launch (run-others) to bootstrap chosen
                               tools + wire ports (HookSource, backend reg, hook-install target)
```

Process count by need: memory-only = memory tool + gate (for the hook flow); PTY-only = 1 (daemon);
full = 3 (daemon + gate + tool gateway), memory a library inside whatever consumes it.

## Pros & Cons

### Pros

- One observability chokepoint. All agent-facing traffic (hooks + tool calls) flows through the gate,
  giving one correlation-tagged view for debugging now and a substrate for future features — the
  reason the gateway routes through the gate even before policy middleware exist.
- One policy point later. The same chokepoint gains policy middleware (validate/firewall/redaction)
  with no caller change — just add middleware to a route.
- Thinnest daemon. The PTY daemon does one thing — own fds and stream bytes — with no second
  protocol (hook ingress) bolted on.
- One lifecycle implementation. `service-host` replaces copy-pasted manifest/socket/signal/
  shutdown/probe code across crates.
- One hook ingress, one MCP front. Removes the duplicate ingress and the gateway-bypass.
- Clear dependency direction. Tools depend only on contracts and shared libs; never on each other
  or on orchestrators. The daemon knows nothing downstream.
- Hot path untouched. PTY bytes stream direct; the gate never adds latency to rendering.
- Genuine composability. Each tool ships and runs standalone; the same binaries compose with no
  code change.

### Cons / Costs

- The gate is a new agent-facing process (hook endpoint + tool route); it must be hardened and its
  routes strictly isolated (tool route != hook endpoint != admin channel).
- Every MCP tool call costs a gate round-trip (in v1 an observe pass-through). Local IPC makes this
  sub-millisecond, but it is a hop on the tool-call path to keep cheap.
- More small crates (two lifecycle libs, the contract surface, core libs); runtime processes are
  daemon + gate + tool gateway, with memory a library.
- New versioned wires to maintain (PTY session subscription; the gate's hook-event subscription;
  the gate tool inbound/outbound); each must stay backward-tolerant within a major version.
- The queue adds a small state machine (status + attempts + reclaim-on-startup) and needs
  idempotent ingest for at-least-once delivery.
- Cross-process freshness: gate-side recall reads committed rows via WAL — eventually-fresh, not
  strictly live. Acceptable for recall; stated, not assumed.
- Migration touches every Rust crate; larger blast radius than a localized fix.

## Scope

### In scope

- New libraries: `service-host`, `process-launch`, `daemon-pty-client` (Rust, PTY wire),
  `gate-client` (Rust, gate hook-subscription wire), and `contracts-rs` mirroring `@athing/sdk` under
  one versioned spec. (No `gate-core`/`memorya-core` crates — see below.)
- The gate: a new router over composable middleware (`seq`/`par`) — **v1 globals: `Observe` (outermost), `Auth`**;
  validate, firewall, redaction are future middleware. The Hook route (hook endpoint -> Observe -> Auth
  -> Normalize via injected adapter -> FanOut by session id) and the Tool route (Observe -> Auth ->
  PassThrough). **Observability** is the `Observe` global — a correlation-bound record per inbound
  (ADR-0012). The gate speaks no tool protocol; normalization is the injected adapter's job. The
  middleware framework + concrete middleware are **modules in the gate binary**; shared wire types
  live in `contracts-rs`.
- The tool gateway (`mcp-gateway-rs`): stays a standard MCP front + external-backend supervisor
  (ADR-0014 unchanged); sends `ToolCall`/`ToolResult` inbounds to `gate.handle()` (v1: observe
  pass-through, fail-open) for unified observability; adopts `service-host`.
- PTY daemon: remove hook ingress; expose only PTY bytes + session control + a session-event
  subscription; adopt `service-host`.
- Memory: `HookSource` port (gate-subscription adapter; stub source for tests); durable capture queue
  - worker; dual-mode recall; the memory tool is the thin bin over `memorya-rs`'s existing library.
- Orchestrators (server, desktop, cli): become thin; bootstrap via `process-launch`; wire ports
  and the hook-install target.

### Out of scope

- Gate policy middleware — `validate`, `firewall`, `redaction` (a route accepts them later; in v1
  the tool route is observe-only and the gateway is fail-open on it).
- An observability exporter/collector — v1 emits ADR-0012-shaped structured records only; wiring an
  exporter is later.
- Daemon upgrade simplification (drain-and-restart replacing fd-handoff; revisits ADR-0010/0011).
  Separate change `daemon-upgrade-drain-restart`. This change assumes the current upgrade path.
- Routing PTY bytes through the gate (explicitly rejected — bytes stay on the direct hot path).
- Compression of internal IPC (explicitly rejected — loopback bandwidth is not scarce; compression
  costs CPU/latency and fights raw-bytes-end-to-end. Network-edge compression, e.g. the server's
  WebSocket to a remote UI, uses the transport's standard mechanism and is out of scope here).
- Multi-user / commercial deployment, API-key auth (remains under Commercial Terms).
- Dynamic / dlopen / WASM plugin loading (rejected — YAGNI for the tool count).
- Running multiple tools inside one process as the default (opt-in dev composition only).
- Windows support (v1 remains macOS/Linux).

## Capabilities

### New Capabilities

- `service-host`: the "run-me" lifecycle contract every long-lived tool uses — directory/manifest
  resolution, atomic manifest write, socket/path binding helpers, signal handling, graceful
  shutdown, liveness probe.
- `process-launch`: the "run-others" contract used only by orchestrators and by the tool gateway over
  its external backends — spawn, adopt-or-spawn-and-wait, restart, spawn-field diffing.
- `gate`: the agent-facing router over composable middleware (`seq`/`par`) — **v1 globals: `Observe`
  (outermost), `Auth`**; validate, firewall, redaction are future middleware. A Hook route (hook
  endpoint -> Observe -> Auth -> Normalize via injected adapter -> FanOut by session id), a Tool route
  (Observe -> Auth -> PassThrough), a hook-event subscription for consumers, **observability** (the `Observe` global — a
  correlation-bound record per inbound, ADR-0012), route isolation, and loopback+token binding. The
  gate speaks no tool protocol and holds no backend registry; normalization is the injected adapter's
  job (the gate is agnostic in code).
- `daemon-session-subscription`: the PTY daemon's versioned public surface — raw byte streaming,
  session control, and a session-event (lifecycle) subscription — with no hook ingress and no
  downstream knowledge.
- `tool-composition`: the composable-tools topology — standalone tools, the "a tool never spawns a
  tool" rule, dual-mode ports, and thin optional orchestrators.

### Modified Capabilities

- `rust-pty-daemon`: becomes pty-only. The "hook ingress capability" requirement is removed (moves
  to `gate`); the daemon exposes a session-event subscription and carries no downstream
  knowledge.
- `memorya-capture`: capture is driven through a `HookSource` port (the gate subscription); the
  enqueue becomes durable and is drained by a proactive worker.
- `memorya-recall`: recall is dual-mode — a standalone standard MCP front, or fronted by the tool
  gateway as a backend when composed — instead of always running its own MCP server.

## Impact

- **Crates:** new `service-host`, `process-launch`, `daemon-pty-client`, `gate-client`,
  `contracts-rs`, and a new **gate** crate (router + middleware + hook/tool routes + observability); the gate's framework and
  the `parse_hook` `AgentAdapter` are **modules in the gate binary** (no `gate-core`; no separate
  adapter crate yet — a per-agent adapter crate only at multi-agent, per ADR-0003). `mcp-gateway-rs`
  stays a standard MCP front + backend supervisor and sends tool inbounds to the gate (v1: observe);
  `daemon-pty-rs` loses hook ingress; `memorya-rs` keeps its existing library and becomes a thin memory
  tool over it (no `memorya-core` crate), consuming canonical `HookEvent`s;
  `apps/server`/`apps/desktop`/`apps/cli` thinned. The TS `parseHook` for hooks retires (the engine
  consumes the canonical event from the gate); the transcript reader and the adapter's
  `parseTranscriptEntry`/`transcriptPath` are removed (ADR-0006 dropped). (`redact-rs` / a redaction
  middleware is a planned later addition to a gate route — out of scope here.)
- **Processes:** the three long-running Rust processes become daemon (pty-only) + gate + tool
  gateway; memory runs as a library inside the memory tool. A memory-only deployment is **memory +
  gate** (the gate is the ingress); memory subscribes to the gate for events.
- **Trust:** one chokepoint (the gate router) for all agent-facing data; the daemon holds no
  untrusted ingress; the tool gateway routes its tool calls through the gate.
- **Contracts:** the PTY daemon wire, the gate's canonical hook-event subscription, and the gate
  tool inbound/outbound become public versioned surfaces mirrored in `contracts-rs` + `@athing/sdk`.
  The canonical `HookEvent` payload is enriched to carry capture fields (prompt content; tool
  name/input/response; turn index) so consumers need no raw-format knowledge.
- **Data:** memory adds a capture-queue table; ingest must be idempotent (dedup by content hash);
  gate-side recall reads via WAL.
- **ADRs:** new ADR for the composable-tools topology + the gate (middleware router); **amends
  ADR-0008** (hook ingress leaves the daemon -> the gate; daemon becomes pty-only); **realizes
  ADR-0005** (the generic ingress calls `parseHook` and routes by session) as the gate's hook flow;
  **supersedes ADR-0006** (transcript read-on-hook dropped; content moves to the `HookEvent`
  payload), which leaves the adapter **single-language** (Rust `parseHook` only) — ADR-0004 stays
  satisfied. **ADR-0014 is unchanged** —
  the tool gateway stays a standard MCP front and merely calls the gate to clean. Relates to
  ADR-0003 (ports-and-adapters) and ADR-0013 (separate detached processes).

## Things to Remember

- Breaking changes are acceptable — pre-v1; no back-compat shims or deprecation aliases.
- The gate is data plane (runtime message flow); the orchestrator is control plane (launch + wire).
  Never merge them. The orchestrator launches the gate.
- PTY bytes NEVER traverse the gate. The gate handles hook and tool inbounds only; bytes stay direct.
- The gate speaks no tool protocol. The tool gateway owns MCP and sends tool inbounds to
  `gate.handle()`; the gate never knows MCP and holds no backend registry.
- The gate's routes must be isolated: a tool-route caller cannot inject hook events into the
  fan-out, and neither agent route can administer the gate.
- Dependency arrows point only to contracts and shared libs. Never tool -> tool, never
  tool -> orchestrator. `daemon-pty-client` is the only Rust code that knows the PTY daemon wire.
- `service-host` owns lifecycle and filesystem coordination only — never transport or protocol.
  The daemon keeps unix-socket binary framing; the gate and tool gateway keep loopback HTTP. The
  moment the host dictates the wire, the abstraction breaks.
- Middleware are composable (`seq`/`par`) and declared at the composition root; the contracts are a
  testable library. A middleware that does I/O (Observe, FanOut) is tested with a fake; pure ones
  (Auth) trivially. Adding policy = add a middleware to a route; the gate is untouched.
- Every message carries a `correlation_id` (distinct from `session_id`) so one action can be followed
  across process hops; it rides all three wires and is bound in logs (extends ADR-0012 bound context).
  This is NOT distributed tracing — spans/exporters/propagation machinery stay deferred per ADR-0012.
- The capture queue is for a single local user: status + attempts + reclaim-on-startup only. No
  retry/backoff/dead-letter/priority machinery.
- Memory is a library with two faces (capture near the hook source; recall behind the MCP front).
  It is not a service and owns no peer-spawning.
- Honor the reliability/operability contract (ADR-0007) in every tool via `service-host`.

## Where to Start

1. Extract `service-host` from the existing duplicated lifecycle code (evidence-based — the
   duplication already exists). Migrate the PTY daemon to `host::run` first to prove the trait.
2. Extract `process-launch` (including `spawn_fields_differ`); move orchestrators' adopt-or-spawn
   onto it; remove peer-spawning from every tool.
3. Strip hook ingress from the PTY daemon; expose the session-event subscription; mirror the wire
   in `contracts-rs` + `@athing/sdk`; publish the Rust `daemon-pty-client`; align the TS engine's
   `proxy.ts` with the versioned spec (not a rewrite).
4. Define the canonical `HookEvent` (typed payload) in `contracts-rs` + `@athing/sdk`; port
   `parse_hook` to an `AgentAdapter` module in the gate binary (no separate crate yet); retire the TS
   `parseHook` for hooks. Build the gate: the framework modules (`Middleware`/`Router`/`Ctx` +
   `seq`/`par`), the `Auth`/`Observe`/`Normalize`/`FanOut` middleware, the Hook route + Tool route
   (PassThrough), the hook-event subscription, and observability (correlation-bound records); isolate
   the routes. Then have `mcp-gateway-rs` send tool inbounds to `gate.handle()` for observability
   (fail-open; gateway otherwise unchanged) and adopt `service-host`.
5. In `memorya-rs`'s library, introduce the `HookSource` port (gate-subscription adapter; stub source
   for tests); the memory tool subscribes to the gate.
6. Add the durable capture queue + worker; convert lazy `embed_pending` flush to a proactive
   drain; make ingest idempotent.
7. Make recall dual-mode; front memory's recall through the tool gateway as a backend when composed.
8. Thin the orchestrators (server, desktop, cli); wire ports + hook-install target; verify each
   tool still runs standalone.

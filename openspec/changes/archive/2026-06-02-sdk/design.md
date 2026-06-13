## Context

`@athing/sdk` is the first package of a-thing: a library that drives a coding-agent CLI (Claude Code first) from a host application, so any UI can integrate it without reimplementing an agent loop or holding credentials. The eventual product is a web UI; this change is the SDK only.

There are three viable ways to wrap a coding-agent CLI, forming a spectrum:

- **Pseudo-terminal (PTY)** — run the CLI inside a fake terminal and exchange raw bytes. The agent runs fully interactive (colors, prompts, live rendering). Output is opaque bytes (great for rendering a terminal, useless for structured queries). Works for any CLI without a per-agent protocol.
- **Headless structured stream** — run the CLI in a non-interactive mode that emits machine-readable events on stdout. Clean typed data, no vendor library, but no live terminal and permissions must be handled out of band.
- **Vendor agent library** — link the agent's own programmatic library. Typed events, but couples the SDK to a per-agent dependency.

The PTY route is chosen as the v1 transport because it delivers the genuine interactive terminal a future UI can render faithfully, and it generalizes to any CLI. There is one engine (`@athing/engine`); transport is a per-session feature inside it — PTY now, a headless stream-json mode later. The two modes are separate internal code paths (not forced under a shared `Transport` interface) that converge on the same canonical event model. Structured content is recovered in the PTY mode by reading the agent's on-disk session transcript.

Constraints: Bun runtime; turbo monorepo already scaffolded; no API key handling; multiple agents in the future, Claude Code now.

## Goals / Non-Goals

**Goals:**

- A single engine (`@athing/engine`) realizing the shared `AgentSession` contract; the PTY transport now, a stream-json transport as a future per-session feature/mode in the same engine. One engine instance hosts many concurrent sessions (mixed modes), with no global state.
- A working `PtyTransport` that drives interactive `claude` and exposes raw-byte I/O + resize.
- Hook-based status normalization to a 5-state model, received over a local HTTP bridge.
- Transcript-derived typed content events so PTY mode still yields structured data — the unifier that makes both transports emit the same content stream.
- A pluggable `AgentDefinition` adapter, with `claudeCode` as the first implementation.
- Raw, unmangled bytes end-to-end and PTY resize propagation, so a terminal UI renders faithfully.
- A ports-and-adapters package layout (`sdk` ports, `engine` machinery, per-agent adapters) so future agents are additive.
- Hybrid adapters: an `AgentDefinition` is declarative config (launch flags, hook-install spec, version range) plus small parse functions where logic varies (`parseHook`, `transcriptPath`, `parseTranscriptEntry`) — so a new agent is mostly a data file with a little code, each part where it is cheapest to maintain, and the engine stays agent-blind.
- A minimal vertical slice — `apps/server` (WebSocket/HTTP bridge + composition root) and `apps/ui` (SPA) — that exercises the ports end to end to validate the architecture.
- Operability fit for unattended use: clean lifecycle/shutdown, bounded interactions (timeouts, backpressure), a typed error taxonomy, an authenticated control plane, independent plane degradation, CLI-version awareness, and minimal session-correlated observability.

**Non-Goals:**

- A production application UI. A minimal `apps/ui` dev harness and the `apps/server` WebSocket/HTTP bridge ARE in scope, solely to bootstrap and visually validate the stack.
- The headless stream-json transport mode (v2 — a per-session feature inside `@athing/engine`, not built here).
- PTY daemon / cross-restart session persistence; persistent session store.
- Additional agent adapters beyond Claude Code.
- A programmatic permission control plane.
- Credential or API-key management.

## Architecture

### Packages & dependency direction (ports-and-adapters)

```
                 ┌──────────────────────────────────────────────┐
                 │  @athing/sdk   (PORTS — types/contracts only)  │
                 │  AgentSession · AgentDefinition · HookEvent ·  │
                 │  event model · status enum · option types      │
                 │  zero deps · zero impl                          │
                 └──────────────────────────────────────────────┘
                     ▲                ▲                 ▲
          implements │      implements│        types    │
                     │                │                 │
        ┌─────────────────────────┐  ┌───────────────────────────┐
        │ @athing/engine           │  │ @athing/adapter-claude-code│
        │ (the machinery)          │  │ (config data + parse fns)  │
        │ deps: sdk                │  │ deps: sdk                  │
        │ NEVER imports an adapter  │  └───────────────────────────┘
        └─────────────────────────┘              ▲
                     ▲                       │
                     └───────────┬───────────┘
                       ┌───────────────────────────────────┐
                       │ apps/server  (COMPOSITION ROOT)     │
                       │ imports the engine; injects adapter │
                       │ exposes a session over WS + HTTP    │
                       │ deps: engine + adapter + sdk        │
                       └───────────────────────────────────┘
                                     ▲ WS (bytes/status/content) + HTTP
                       ┌───────────────────────────────────┐
                       │ apps/ui  (react-router SPA)         │
                       │ xterm panel + status + content view │
                       │ deps: sdk types + network           │
                       └───────────────────────────────────┘

dependency rule: deps point INWARD to sdk. engine never imports an adapter;
the adapter is injected at apps/server (DI); apps import the engine directly.
add a new agent -> new adapter (data); engine + apps unchanged.
```

### Runtime — three planes + engine internals

```
        ┌─────────────────────── @athing/engine ───────────────────────────┐
        │  engine: consumes AgentDefinition config + calls its parse fns      │
        │                                                                    │
        │  DRIVE plane (PtyTransport, node-pty)                              │
        │    spawn `$SHELL -lc 'exec claude --session-id <id> ...'` (no noise)│
        │    send(text, gated) / input(bytes) / interrupt / resize / kill    │
        │    onData(bytes)                                                    │
        │        │ raw bytes (faithful TUI)                                  │
        │                                                                    │
        │  INGRESS (generic receiver, Bun.serve)                            │
        │    install hook(once) per adapter DATA -> agent fires -> POST      │
        │    -> verify token -> validate -> adapter.parseHook() -> route id  │
        │    ════════ emits HookEvent ════════  (the contract seam, D15)     │
        │            │ HookEvent (normalized contract enum)                  │
        │    ┌───────┴────────────────────────────┐                         │
        │    ▼ (consume HookEvent, transport-blind)▼                         │
        │  STATUS                        CONTENT (read-on-hook)              │
        │   enum->status (GENERIC)         PostToolUse/Stop -> read .jsonl   │
        │   {IDLE|WORKING|                 delta -> adapter.parseEntry()     │
        │    WAITING_INPUT|DONE}           -> tool_use·edit·usage; exit=read │
        │        │ status                      │ content                     │
        └───────────────────────────────────────────────────────────────────┘
                  │ AgentSession fans out: onData · onStatus · onContent
                  ▼
            apps/server (WS) ──▶ apps/ui (xterm + panels)
```

### Turn data-flow

```
caller            AgentSession        PtyTransport        claude          hooks/disk
  │ start(adapter,opts) │ install hook(once) ──────────────────────────▶ settings.json
  │────────────────────▶│ spawn $SHELL -lc 'exec claude --session-id...' ▶│ boots
  │                     │                              │── SessionStart ─▶ POST+token ▶ IDLE (ready)
  │ send("refactor") ──▶│ (gated on IDLE) paste+submit ▶│ works           │
  │                     │                              │── UserPrompt ───▶ ▶ WORKING
  │ onData(bytes) ◀─────│◀── pty bytes ────────────────│ (live TUI)       │
  │                     │                              │── PostToolUse ──▶ ingress▶HookEvent▶ read .jsonl delta
  │ onContent(tool) ◀───│◀──────── transcript delta ───────────────────── │
  │ interrupt() ───────▶│ send Esc ────────────────────▶│ (cancel turn)   │
  │                     │                              │── Stop ─────────▶ ▶ IDLE (+final read)
  │ onStatus(...) ◀───────│  kill(): SIGTERM->grace->SIGKILL -> Exited{code,signal}
```

(every POST -> ingress verifies token, calls `adapter.parseHook` -> `HookEvent`, then status + content consume it — D15.)

### Cross-cutting (the harness contract)

```
auth        user's own subscription login · no API key · 1 sub = 1 user (bring-your-own)
adapter     hybrid: DATA (launch flags · hook-install spec · version range)
                    FUNCTIONS (parseHook · transcriptPath · parseTranscriptEntry)
engine      instance-based (createEngine()), no globals -> many concurrent sessions
reliability TERM->grace->KILL · timeouts · typed errors (incl. NotAuthenticated) ·
            token-auth bridge · backpressure · plane degradation · CLI version check ·
            session-correlated logs
bytes       raw end-to-end, no ANSI strip / no re-decode · resize -> PTY ioctl
deps        node-pty v1.1.0 · Bun.serve · Bun.file · Bun.spawn · valibot · bun test
scope       macOS/Linux v1 · PTY now, stream-json a future per-session mode
```

## Decisions

### D1: PTY as the v1 agent transport

Drive interactive `claude` through a pseudo-terminal so the host gets the genuine TUI and full interactivity.

- **Why:** The goal is to harness everything the agent exposes and show the live terminal in a future UI; PTY bytes render faithfully in a terminal emulator, and the approach works for any CLI with no per-agent protocol.
- **Alternatives:** (a) headless structured stream — cleaner typed events, no vendor library, but headless with no interactive TUI; deferred to v2 as a stream-json transport mode inside the engine. (b) vendor agent library — typed events but couples to a per-agent dependency; rejected for the no-vendor-SDK goal.
- **Trade-off accepted:** PTY output is opaque bytes — mitigated by D4.

### D2: One engine; transport is a per-session feature (no `Transport` interface)

There is a single `@athing/engine`. Transport is selected per session (PTY now; a headless stream-json mode later). The two modes are **separate internal code paths**, not implementations of a shared `Transport` interface — they converge only on the canonical event model (data/status/content) the engine emits. No separate engine packages.

- **Why:** PTY and stream-json have fundamentally different shapes — PTY is raw bidirectional bytes + interactive + resize; stream-json is typed events + headless + no resize. A shared `Transport` interface fitting both would be a leaky lowest-common-denominator, so we don't force one; the engine just has two code paths. Keeping them in one engine (one package) is simpler to build, version, and run — and a UI can mix modes across concurrent sessions in one engine instance (D16).
- **Consequence:** Apps depend on the single `@athing/engine`; the session start option chooses the transport mode. The `AgentSession` contract is uniform across modes; the sdk exposes no transport port.
- **Alternatives:** A shared `Transport` interface — leaky across such different transports; rejected (two internal code paths instead). Per-transport engine packages — more packages and a core extraction for no v1 benefit; rejected in favor of one engine.

### D3: Status from lifecycle hooks, via ingress + the `HookEvent` contract

Status is sourced from the agent's native lifecycle hooks, but the engine is split at the `HookEvent` contract (D15): a generic **ingress** component runs a `127.0.0.1` receiver, hands each raw payload to the adapter's `parseHook` function to get a normalized `HookEvent`, and a **status mapper** maps the event's contract type to `{ IDLE | WORKING | WAITING_INPUT | DONE }`. The status mapper never touches HTTP; the ingress never interprets status.

- **Why:** PTY bytes carry no reliable machine-readable state, and scraping the terminal is fragile. The agent's own lifecycle hooks are an authoritative signal; localhost HTTP is agent-agnostic and trivial to receive. Splitting ingress (how events arrive) from status (what they mean) keeps status logic transport-blind and unit-testable. The only agent-specific step is `parseHook` (D12); because it normalizes to the fixed contract enum, the status mapper is **generic** (no per-adapter table).
- **Alternatives:** (a) scrape terminal output for state — fragile, breaks on UI changes. (b) unix socket / file transport — more plumbing, less portable than localhost HTTP; remains a future `HookEvent` producer behind the same contract. (c) derive status from the transcript instead of hooks — collapses planes but loses real-time latency and reliable `WAITING_INPUT`; rejected as the default, but becomes just another `HookEvent` producer if ever wanted. The transcript is content-only (D4); status comes from hooks.
- **Generic contract enum -> status mapping** (engine, fixed — the contract enum has defined meaning):

```
SessionStart                       -> IDLE          (booted)
UserPromptSubmit · PostToolUse     -> WORKING
PermissionRequest                  -> WAITING_INPUT
Stop                               -> IDLE          (turn done)
SessionEnd                         -> DONE
```

The `claude-code` adapter's job is only to register the hooks (data: `SessionStart`, `UserPromptSubmit`, `PostToolUse` matcher `*`, `PermissionRequest`, `Stop`, `SessionEnd`) and to `parseHook` Claude's raw payloads into these contract types.

- **Mechanism:** a small notify command (from the adapter's hook-install data) reads the hook payload and POSTs it to the injected loopback URL; the engine's ingress verifies the per-session token (D13.4), validates the envelope, calls `adapter.parseHook` to produce a `HookEvent`, and routes by id (D11) before dispatching it to the status mapper and content reader. The notify command is adapter data; `parseHook` is an adapter function; the receiver is generic engine code.

### D4: Structured content from the on-disk transcript (read-on-hook)

A transcript reader reads new lines from the agent's session JSONL file and emits typed events (tool_use, edits, usage, cost). It is **triggered by the hook plane** — on `PostToolUse` it reads the delta (the tool entry is already written by then), and on `Stop` it does a final read — so there is no file watcher and no poll loop. As a fallback, it also reads once on process exit (D13.1) so end-of-session content is captured even if the hook plane is degraded. The session id is known up front (the engine sets it via `--session-id`, D8 note) so the transcript path is resolvable before the first hook.

- **Why:** Recovers structured content without a vendor library and without changing transport. It normalizes to the **same** typed shape the future structured transport will emit, so consumers get one content model regardless of transport. Reusing hook events as the read trigger removes the flaky cross-platform `fs.watch` and any polling timer.
- **Trade-off / coupling:** content granularity is per-tool / per-turn (not sub-tool streaming) — acceptable because the PTY already shows live activity. Content shares the hook trigger with status, so a dead hook plane reduces content to the exit-time read (see D13.6).
- **Alternatives:** `fs.watch` — lowest latency but flaky across macOS/Linux; rejected. Polling — robust but adds a per-session timer and latency; rejected in favor of the hooks we already have. Parse PTY bytes for content — fragile and lossy; rejected.

### D5: Permissions punted

Launch the agent with permission prompts disabled (`--dangerously-skip-permissions`); if the agent still prompts, it renders in the terminal and the user answers there.

- **Why:** Keeps v1 simple and fully interactive; a structured permission control plane is a known later option not needed now.
- **Risk:** Skipping permissions is unsafe outside a trusted/sandboxed context — see Risks.

### D6: In-process PTY (no daemon yet)

Run the pseudo-terminal inside the SDK process.

- **Why:** Daemonizing PTYs to survive host restarts is a UI/server persistence concern, out of scope here.
- **Alternative:** PTY daemon now — premature; it can be added inside `@athing/engine` later without changing the `AgentSession` contract.

### D7: Raw-byte fidelity + resize propagation as hard requirements

No ANSI stripping and no intermediate text re-decode on the byte path; resize propagates to the PTY.

- **Why:** Any re-decode mangles multibyte sequences and garbles the terminal; a missing resize breaks wrapping and cursor placement. Required for a faithful future terminal render.

### D8: Ports-and-adapters packaging (sdk / engine / adapters / apps)

Split into three packages plus two apps, with a strict dependency direction:

```
@athing/sdk                ports + types only, zero deps, zero impl
                             AgentSession · AgentDefinition · HookEvent contracts
                             · event model · status enum · option types
@athing/engine             the engine (the machinery), depends on sdk
                             AgentSession impl · PtyTransport · hook ingress · status mapper
                             · transcript reader · drives it all from the injected AgentDefinition
                             NEVER imports a specific adapter
@athing/adapter-claude-code  a hybrid AgentDefinition (config data + parse fns), depends on sdk
apps/server                composition root: depends on sdk + engine + a concrete adapter;
                             imports the engine, injects the adapter; exposes a session over WS + HTTP
apps/ui                    react-router SPA: depends on sdk types + the network; no code dep on engine
```

- **Why:** The SDK is the contract future agent implementations build against; keeping it implementation-free makes "support all agents" mean "add an adapter package." The engine depending only on the ports (and never on an adapter) keeps the core agent-agnostic; the adapter is injected at the `apps/server` composition root (dependency injection). Apps are the delivery mechanism and the architecture's first end-to-end test.
- **Alternatives:** (a) One package with internal modules — simplest to consume, but blurs the port/impl/adapter boundary and makes the multi-agent seam implicit; rejected in favor of an explicit contract. (b) Engine imports the adapter directly — removes the composition root but couples the core to a specific agent; rejected.
- **Note (Claude Code specialization of D9/D11):** the SDK generates a session uuid and the adapter passes it via `--session-id`, so the agent adopts it as its own id; that one id then drives hook routing, transcript path, and resume. The env-injected id is the fallback for agents that cannot accept a caller-chosen id.
- **Future (per D2):** a second transport (stream-json) lands as a per-session mode inside `@athing/engine` — a second internal code path, not a new package.

### D9: Install hooks once, scope per session via injected env

Register a static hook command in the agent's settings a single time; differentiate sessions by injecting the hook-bridge URL and session id into each PTY's environment at spawn.

- **Why:** Avoids a settings-file write on every session (and the concurrent-write race that comes with it) and is crash-safe — there is no per-session teardown that can leave stale hooks.
- **Cost:** One hook command persists in the user's settings; the SDK MUST provide an explicit uninstall path.
- **Alternative:** Install per-session and remove on teardown — clean but races on the settings file across concurrent sessions and leaks hooks on crash; rejected.

### D10: Minimal resume in the v1 public API

Support `start({ resume: sessionId })`, which relaunches the agent with its resume flag using the session id already captured from the session-start hook.

- **Why:** The session id is already captured for the content plane, so resume is a one-flag addition that buys multi-turn continuity across a process restart.
- **Scope:** Only the relaunch path; the session store / daemon persistence stays deferred.

### D11: One shared hook bridge, multiplexed by session id

Bind a single `127.0.0.1` listener and route each callback to the owning session by the session id in the payload.

- **Why:** One port regardless of session count; aligns with the status capability's existing "route callback to the owning session by identifier" requirement; scales cleanly to many concurrent sessions.
- **Alternative:** One listener per session — trivial 1:1 routing but consumes a port per session and adds bind/teardown bookkeeping; rejected.

### D12: Hybrid adapters (declarative config, functions for parsing)

An `AgentDefinition` is mostly declarative data, with a few small functions where logic genuinely varies per agent:

```
DATA (config — stable, no logic):
  launch:      command + args template (placeholders like {id}, {resume}) + flags
  hookInstall: { settingsPath, commandTemplate, events[] }
  cliVersion:  supported version range
FUNCTIONS (logic — agent-specific):
  parseHook(raw): HookEvent                 raw hook payload -> normalized contract event
  transcriptPath(sessionId, cwd): string    incl. the cwd-encoding rule
  parseTranscriptEntry(line): Content|null  per-entry-type extraction (tool_use/edit/usage)
```

- **Why:** Declarative is easy and clear for stable structural config (flags, hook events, settings path, version range) — a new agent edits data there. But parsing (hook payloads, transcript entries) and the cwd-encoding rule are _logic_; forcing them into a data schema builds a config-DSL + a growing engine interpreter (indirection, hard debugging, an expressiveness ceiling). A plain function is unit-testable (`parseHook(sample) === expected`), debuggable with a breakpoint, and unbounded in expressiveness. Hybrid puts each concern where it's cheapest to maintain.
- **Consequence:** The engine stays Claude-blind — it calls `adapter.parseHook` and receives a `HookEvent`; it never knows the agent's payload shape. Because `parseHook` normalizes to the **fixed contract enum**, the engine maps the enum -> status **generically** (no per-adapter status table). The adapter functions are reached through the `AgentDefinition` contract (sdk) and the concrete adapter is injected at the composition root, so the engine still never imports a specific adapter (D8).
- **Trade-off:** Adapters are now code (a small module + tests), not pure data — accepted; the cross-language motivation for pure data is gone, and code is the more maintainable choice for the parsing parts.
- **Alternatives:** Pure-data declarative — nice for config but turns parsing/encoding into a hard-to-maintain config-DSL + interpreter; rejected. Fully functional (everything code) — fine, but loses the clarity of declarative config for the stable parts; the hybrid keeps that.

### D13: Reliability & operability (the harness contract)

These are operability requirements every `Engine` impl must honor; they slot into the existing planes, not the architecture.

- **D13.1 Process lifecycle & graceful shutdown.** `kill()` SHALL escalate SIGTERM -> grace period -> SIGKILL. The engine SHALL capture the agent's exit code/signal, emit a terminal `Exited { code, signal }` event, reap the child, and clean up the PTY and per-session bridge state on both normal exit and crash. No leaked processes or orphaned PTYs.
- **D13.2 Timeouts on every external interaction.** Bounded startup (agent fails to boot), shutdown grace, and idle timeouts. A timeout produces a typed error and a defined transition, never an indefinite hang.
- **D13.3 Typed error taxonomy.** Errors are a closed set, not strings: `BinaryNotFound`, `NotAuthenticated`, `SpawnFailed`, `HookInstallFailed`, `TranscriptUnavailable`, `TransportClosed`, `Timeout`, `VersionUnsupported`. Surfaced on the canonical event model so callers can branch.
- **D13.4 Authenticated hook bridge.** The bridge is loopback-only AND authenticated: the engine mints a per-session secret, injects it via env + the hook URL, and verifies it on every callback. Unauthenticated or mismatched callbacks are rejected. Closes local-process spoofing of lifecycle state.
- **D13.5 Backpressure / bounded buffering.** Per-session output buffering is bounded with pause/resume on the PTY when a consumer is slow (or an explicit, logged drop policy). No unbounded buffering.
- **D13.6 Independent plane degradation.** A failed plane SHALL NOT kill the session. If the transcript is unavailable, content degrades while DRIVE + STATUS continue. Because content is triggered by the hook plane (D4 read-on-hook), a dead hook plane degrades BOTH status and live content; content then falls back to a single read on process exit (D13.1), and DRIVE continues regardless. All degradation is reported via a typed error, not a crash.
- **D13.7 CLI version awareness.** An `AgentDefinition` declares a supported CLI version range. The engine detects the installed CLI version and emits `VersionUnsupported` (refuse or warn) on mismatch, so an upstream CLI update never silently breaks status/content.
- **D13.8 Minimal observability.** Session-correlated structured logs (every line tagged with the session id) and an opt-in raw-I/O capture / debug-record mode for diagnosing a black-box CLI.

- **Why grouped:** these are the difference between a demo wrapper and an unattended harness; mirrors process-supervisor and LSP/DAP host norms — bounded interactions, capability/version negotiation, clean teardown, typed protocol errors.

### D14: Dependencies & tooling (Bun-first)

Library choices live in the engine and apps only; `@athing/sdk` and the adapters stay dependency-free by contract (D8/D12). The project is Bun-first, so built-ins are preferred over third-party wherever they suffice.

```
@athing/sdk                ZERO runtime deps (types/contracts only)
adapters                   depends on sdk only (config data + small parse fns)
@athing/engine
  PTY binding              node-pty v1.1.0 (pinned) — runs under Bun
  hook bridge HTTP         Bun.serve
  transcript read          Bun.file (read delta on PostToolUse/Stop hook; no watcher/poll)
  process / version probe  Bun.spawn (`claude --version`) + small semver compare
  adapter/message validate  valibot (project-standard validation library)
  logging                  thin structured wrapper (no heavy logging dep)
apps/server                Bun.serve (HTTP + WebSocket); no express, no ws
apps/ui                    React + react-router + xterm.js (+ fit addon); Bun bundler, no vite
test                       bun test
```

- **Why node-pty v1.1.0:** the established, battle-tested native PTY binding; proven under Bun. Pinned for reproducibility.
- **Why Bun built-ins elsewhere:** the project mandates `Bun.serve`, native `WebSocket`, `Bun.file`, and `Bun.spawn`; using them removes express/ws/execa/chokidar entirely and keeps the dependency surface tiny.
- **Validation standard:** valibot is the project-standard validation library — used to validate adapter config data, the `HookEvent`s produced by `parseHook`, and WS wire messages, failing fast on malformed input. All runtime validation in the engine and apps goes through it.

### D15: The `HookEvent` contract is the engine's lifecycle seam

The engine consumes lifecycle exclusively as a normalized `HookEvent` contract (in `@athing/sdk`) — roughly `{ sessionId, type: SessionStart|UserPromptSubmit|PostToolUse|PermissionRequest|Stop|SessionEnd, payload? }`. Inside the engine, the ingress (D3) is the producer — it calls `adapter.parseHook` to build `HookEvent`s; the status mapper and content reader are the consumers. The consumer side knows nothing about HTTP, tokens, or raw payload shapes.

- **Why:** This is the seam that makes "the engine doesn't care how hooks arrive" structurally true. Any producer — the loopback HTTP receiver (v1), a unix socket, a transcript-derived source, a stream-json source, or a test stub calling `dispatchHook(event)` — feeds the same contract. Status and content logic are tested by handing them `HookEvent`s directly, with no I/O.
- **Consequence:** The auth token and envelope validation live on the **producer** side (ingress, D13.4); the agent-specific raw->contract parsing is `adapter.parseHook` (D12); the engine trusts the `HookEvent`s it receives. Whoever can call `dispatchHook` is inside the trust boundary — the HTTP receiver guards that boundary with the token.
- **Boundary:** the agent-specific knowledge is split — hook-install config is adapter **data**, raw->`HookEvent` parsing is an adapter **function** (D12); the generic act of receiving, authenticating, and dispatching is engine code (`hook-ingress`).

### D16: Engine is instantiable; sessions run concurrently (mixed modes)

The engine is created via a factory (`createEngine()`) returning an isolated instance with its own session registry and resources; it holds **no module-level/global mutable state**. One engine instance hosts many concurrent sessions, each possibly a different transport mode (PTY now, stream-json later), and a UI consumes them uniformly through the shared `AgentSession`/event contracts.

- **Why:** A UI may need an interactive (PTY) session and a headless (stream-json) session at the same time — different sessions, different agents. The shared contract makes the UI mode-blind; no global state keeps concurrent sessions isolated (and lets a host spin up more than one engine instance if it ever wants to).
- **Requirements:** (1) factory-created instances, no singletons; (2) per-session resources are isolated (the shared loopback receiver routes by session id, D11); (3) hook installation is idempotent (D9); (4) `shutdown()` releases only that instance's resources.
- **Non-use:** running PTY _and_ stream-json for the _same_ logical session means two `claude` processes doing the same work — wasteful; the intended use is distinct concurrent sessions, not duplicating one.

### D17: Interactive PTY session lifecycle

Driving the _interactive_ TUI (not headless `-p`) has a lifecycle the engine must handle explicitly:

- **First-run blockers.** Before the agent is ready, an interactive launch can hit blocking dialogs: workspace **trust**, onboarding/theme, or — if not logged in — the **login** flow. The engine SHALL launch with flags/preconditions that skip trust/onboarding where possible, treat a session that is not ready within the startup timeout (D13.2) as a typed error, and detect a not-logged-in state as `NotAuthenticated` (D13.3) rather than hang.
- **Ready gating.** A session is "ready" for prompts only after `SessionStart` (status IDLE). `send()` issued before ready, or while `WORKING`, is queued until the next IDLE (bounded queue; overflow -> typed error). The first prompt waits for ready.
- **`send` vs `input`.** `send(text)` submits a prompt _turn_ — delivered via bracketed paste (multi-line safe) then the submit key; gated on readiness. `input(bytes)` writes raw keystrokes verbatim (answering in-TUI prompts, arrows, y/n) with no gating.
- **`interrupt()`.** Cancels the _current turn_ (sends the agent's interrupt key, e.g. Esc) while keeping the session alive — distinct from `kill()`, which terminates it.
- **Clean launch (no shell noise).** Spawn the user's login shell as `$SHELL -lc 'exec <claude> ...'`: the login shell loads the user's environment (PATH, version managers, rc files) and `exec` replaces it with `claude`, so the byte stream is the agent TUI only — no shell prompt or echoed command.
- **Binary resolution.** Resolve `claude` via `CLAUDE_CODE_EXECUTABLE`, then the login-shell PATH (from the `-l` above), then common install locations; failure -> `BinaryNotFound` (D13.3). The login-shell launch makes PATH resolution match the user's interactive environment.
- **IDLE semantics.** Under `--dangerously-skip-permissions`, `PermissionRequest`/`WAITING_INPUT` rarely fires; a clarifying question from the agent simply ends the turn (`Stop` -> IDLE). So IDLE means "awaiting the user" (done **or** asking) — the UI should treat IDLE as the input-ready state, not strictly "finished."

## Risks / Trade-offs

- Opaque PTY bytes (no structure) -> mitigated by the D4 transcript content plane; raw bytes are a feature for terminal rendering, not a defect.
- Skipping permissions is unsafe in untrusted contexts -> document clearly; intended for trusted local/sandboxed use; a structured permission control plane is left as a future option.
- The PTY binding (node-pty v1.1.0) is a native addon -> accept the native dependency, pin the version, and verify it builds/loads under Bun in CI.
- Mutating the user's agent settings to register hooks -> merge non-destructively, scope hooks to the SDK, provide clean uninstall; never clobber existing user hooks.
- Transcript timing -> with read-on-hook (D4), the first read happens on `PostToolUse`/`Stop`, by which point the file exists; the reader must still treat a missing/short file as empty content (typed `TranscriptUnavailable`) rather than erroring, and track a byte offset to read only the delta each time.
- Hook-bridge port binding -> bind an ephemeral port on `127.0.0.1`, inject the resolved URL into the PTY env at spawn, and handle port-in-use.
- Encoding hops mangle the terminal -> enforce a byte-aligned path end-to-end (D7), covered by a no-encoding-hops test.
- Auth / billing / ToS (subscription model) -> the SDK rides the user's own Claude login, no API key. **Constraint: one subscription = one user (bring-your-own-login).** Individual local use is supported; a multi-user service on a single subscription violates Anthropic's terms (no third-party claude.ai login; no account sharing) and would require API keys under the Commercial Terms (pay-as-you-go). Billing nuance (from Jun 15 2026): PTY _interactive_ draws the larger interactive plan limits, while `claude -p`/SDK draws a separate, capped Agent SDK credit — so PTY is cheaper/roomier on a subscription, but automating the interactive transport is the ToS-grayer path Anthropic reserves for human use and may police. `claude -p`/SDK is the officially-blessed but metered path. Implication: PTY-first for individual subscription use; API-key/Commercial for any multi-user deployment.
- Unauthenticated local control plane -> any local process could spoof lifecycle events -> per-session secret token verified on every hook callback (D13.4).
- Hook callbacks over HTTP are at-least-once (the script may retry) -> status application must be idempotent so duplicate callbacks do not corrupt state.
- Upstream CLI changes its hook names or transcript schema -> silent breakage -> adapter declares a CLI version range and the engine detects/refuses on mismatch (D13.7); guard with golden-fixture contract tests.
- Slow consumer + chatty agent -> unbounded memory -> bounded buffer with PTY pause/resume or a logged drop policy (D13.5).
- Leaked processes / orphaned PTYs on crash -> SIGTERM->grace->SIGKILL escalation, exit capture, and teardown on all exit paths (D13.1).
- UI reconnect needs recent output -> a session-scoped replay buffer is required by `apps/server`; deferred from the SDK core, noted as a harness gap to build with the WS layer.
- Transcript rewrite/truncation (e.g. `/compact` or session edits) -> byte-offset tailing (D4) would read garbage -> detect offset > file size or an inode/identity change and reset, re-reading from the start.
- Observability raw-I/O capture (D13.8) can record secrets the user types/pastes (tokens, passwords) -> make capture opt-in, redact where feasible, and warn explicitly; never on by default.
- Notify command portability -> the hook command POSTs to the loopback bridge; `curl` may be absent -> use a portable poster (e.g. `bun -e`/node) rather than assuming `curl`.

## Constraints

- Platform scope: macOS and Linux for v1 (login-shell + node-pty). Windows (ConPTY, no login-shell idiom) is out of scope for v1.

## Migration Plan

Greenfield; nothing to migrate. Rollout = add `packages/sdk` to the existing turbo workspace and its single native dependency. Rollback = remove the package. The hook installer must offer a clean uninstall path so removing the SDK leaves the user's agent settings as they were.

## Open Questions

All prior open questions are resolved: hook install hygiene -> D9 (install once + env scoping); resume in v1 -> D10 (minimal resume); hook-bridge port model -> D11 (one shared bridge, multiplexed by session id).

No in-force ADRs to revisit (greenfield); the adr step should record the durable decisions: transport route (D1), one engine with transport as a per-session feature (D2), content unifier (D4), ports-and-adapters packaging (D8), hybrid adapters — declarative config + parse functions (D12), the reliability/operability harness contract (D13 — notably authenticated control plane, typed error taxonomy, CLI version awareness, and clean lifecycle), the `HookEvent` lifecycle seam (D15), an instantiable engine with concurrent mixed-mode sessions (D16), and the interactive PTY session lifecycle (D17 — ready-gating, interrupt, first-run blockers, clean exec launch).

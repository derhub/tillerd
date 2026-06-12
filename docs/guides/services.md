# Services & Libraries

A map of the moving parts: what each one is, why it exists, and how they find
each other. Start here when the architecture feels hard to follow.

> **0.x is terminal-only.** The agent surface — and the hook traffic it produces — is
> deferred to 1.0.0 ([ADR-0027](../adr/0027-zero-x-is-terminal-only-agent-surface-deferred.md)).
> The gate, hook ingress, memorya, and mcp-gateway stay as shared infrastructure. This
> doc describes the full design and flags what is dormant until the agent surface returns.

## The big picture

The backend is one Rust library — the **orchestrator** — embedded in-process by the
host (the desktop today, a server later, [ADR-0022](../adr/0022-workspace-session-container-above-the-engine.md)).
It owns the workspace and the surface runtime, and it is the *client* of the two
long-lived singletons it also supervises: **daemon** (owns terminals) and **gate** (the
agent trust boundary). Everything else is a library they share or an app that drives them.

```
              host (desktop / server)        composition root
                    │ embeds
                    ▼
              orchestrator                    the backend: workspace + surface-runtime,
              (library, in-process)           client + supervisor of the two singletons
                │ adopt-or-spawns + drives
        ┌───────┴────────┐
        ▼                ▼
  ┌───────────┐    ┌───────────┐
  │  daemon   │    │   gate    │
  │  (PTY)    │    │ (ingress) │
  └─────┬─────┘    └─────┬─────┘
        │ raw bytes      │ hook events (fan-out)
        ▼                ▼
   surface-runtime    memorya
   (in orchestrator)
        │ bytes + status (EventSink)
        ▼
        UI
```

The hot path — raw terminal bytes — goes **daemon → orchestrator surface-runtime → UI**,
over the orchestrator's `EventSink` ([ADR-0024](../adr/0024-surface-runtime-owns-the-pty-proxy-per-surface.md)).
It never touches the gate. The gate only sees agent lifecycle hooks (session start, tool
use, stop), which it normalizes and fans out to whoever subscribed — today just
**memorya** (knowledge capture). In 0.x no agent runs, so that hook fan-out is idle; the
stack stays so 1.0.0's agent surface plugs straight in.

---

## Source of truth: the `~/.tillerd/` directory

There is **no central registry**. Services discover each other by reading well-known
files in `$TILLERD_DIR` (default `~/.tillerd/`). That directory _is_ the source of truth.

| File          | Written by   | Read by                                | Holds                                                                                              |
| ------------- | ------------ | -------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `daemon.sock` | daemon       | orchestrator (+ daemon clients)        | the daemon's PTY control socket; binary-framed                                                     |
| `daemon.json` | daemon       | orchestrator (supervisor)              | `{pid, version}` — is the daemon already running?                                                  |
| `gate.sock`   | gate         | gate-notify, memorya, mcp-gateway      | the gate's single front door; every connection opens with a route preamble (length-prefixed frame) |
| `tillerd.db`  | orchestrator | orchestrator                           | the product store — projects / sessions / surfaces, rusqlite ([ADR-0023](../adr/0023-workspace-data-model-and-two-level-id.md)) |

These names live in exactly one place: the **tillerd-paths** crate
([ADR-0025](../adr/0025-tillerd-paths-as-runtime-layout-source-of-truth.md)). Every
service and host resolves the runtime dir (`$TILLERD_DIR` → `~/.tillerd`), the
socket / manifest / store paths, and its service-binary locations through it — no string
literals scattered across crates.

The gate exposes **one** socket. Each connection's first frame is a **route preamble**
— `{ route, session, token?, wireVersion }` — and the gate demultiplexes on `route`:

| Route       | Opened by             | Credential              | After the preamble                                          |
| ----------- | --------------------- | ----------------------- | ----------------------------------------------------------- |
| `hook`      | gate-notify           | per-session token       | raw hook payload frames, fire-and-forget                    |
| `tool`      | mcp-gateway           | per-session token       | tool-call IPC, request/response                             |
| `subscribe` | memorya               | none                    | ready → server-push hook-event stream                       |
| `admin`     | orchestrator _(1.x)_  | admin token (≠ session) | register / deregister a session                             |
| `mcp`       | a stdio↔socket bridge | per-session token       | the stream upgrades to the gate's own MCP tools             |

Two routes have no live caller in 0.x and return with the agent surface: `admin` (the
orchestrator registers a session so its hooks authenticate) and `mcp` (the bridge that
fronts the gate's own MCP tools is deferred until a consumer exists,
[ADR-0018](../adr/0018-gate-single-route-multiplexed-socket-mcp-socket-only.md)). The
gate's `mcp` route is the gate's *own* tools — distinct from the standalone **mcp-gateway**
service, which aggregates many MCP servers behind a standard front.

**How "is it running?" is answered** (adopt-or-spawn, run by the orchestrator for each
supervised singleton):

1. Read the manifest file. Missing → spawn.
2. Is the PID alive? (`signal(0)`) No → spawn.
3. Does the version match exactly? No → spawn a replacement.
4. Is the socket reachable? No → spawn.
5. All yes → **adopt** the running instance.

This is why a desktop restart reuses the live daemon instead of starting a second one.

---

## Services (long-lived processes)

### daemon (`apps/daemon-pty`)

- **What** — owns every terminal (PTY master fd). One daemon, N sessions.
- **Why** — terminals must survive app restarts and upgrades. A detached daemon keeps
  them alive while the UI comes and goes.
- **Does** — spawn / kill / resize sessions; stream raw bytes to its client. Knows
  nothing about hooks, agents, or downstream consumers.
- **Talks** — binary framing (4-byte length + payload) over `daemon.sock`.

### gate (`apps/gate`)

- **What** — the single front door for agent-facing traffic. The trust boundary. One
  Unix socket (`gate.sock`), no TCP port; binds nothing else.
- **Why** — one place to authenticate, normalize, and fan out everything an agent emits,
  so no other service has to.
- **Does** — accepts a connection, reads its route preamble, applies the
  route→credential policy, then runs the route. The `hook`/`tool`/`mcp` routes feed a
  fixed middleware pipeline; `subscribe` streams events; `admin` mutates the registry;
  `mcp` upgrades the stream to the gate's own MCP tools.
- **Pipeline** (hook/tool/mcp routes):

  ```
  route preamble → demux (route→credential) → Observe → Auth → [route-specific]
                                                          Hook → Normalize → FanOut
                                                          Tool/Mcp → PassThrough
  ```

  - **Demux** — reads the preamble; one policy maps each route to its credential
    (session token for hook/tool/mcp, admin token for admin, none for subscribe).
  - **Observe** — times and logs the request.
  - **Auth** — constant-time token check against the session registry.
  - **Normalize** — calls the adapter's `parse_hook()` → canonical `HookEvent`.
  - **FanOut** — publishes the event to every subscriber for that session.

- **Config** — environment variables only (`TILLERD_GATE_ADMIN_TOKEN`,
  `TILLERD_GATE_QUEUE_CAP`, …). No config file, no port.

### memorya (`apps/memorya`)

- **What** — local knowledge layer. Captures conversation + tool activity, makes it
  searchable.
- **Why** — recall across sessions without sending anything to a server.
- **Does** — subscribes to gate hook events (`subscribe` route), chunks + embeds them
  into SQLite, and serves search (exact term + vector) as a first-party backend of the
  mcp-gateway.

### mcp-gateway (`apps/mcp-gateway`)

- **What** — an MCP aggregator: many MCP servers behind one standard MCP front
  ([ADR-0013](../adr/0013-mcp-gateway-as-standalone-detached-daemon.md)/[0014](../adr/0014-mcp-gateway-front-is-standard-mcp-only.md)/[0015](../adr/0015-mcp-gateway-implemented-in-rust-on-mcp-sdk.md)).
- **Why** — an agent gets one MCP endpoint; first-party tools (memorya search) and
  user-configured backends sit behind it without the agent knowing the difference.
- **Does** — supervises its backend MCP servers and routes tool calls to them. When
  composed (session identity in the env) it forwards each tool call through the gate's
  `tool` route, so the call crosses the trust boundary; standalone it forwards directly
  without the gate.

### gate-notify (`apps/gate-notify`)

- **What** — the hook client. A tiny native binary the agent execs on each lifecycle
  event (not a daemon — one shot per hook, then exits).
- **Why** — `curl` can't write length-prefixed frames; a small fast binary keeps the
  agent's hot path cheap (~1-2 ms spawn).
- **Does** — reads the hook payload on stdin, opens `gate.sock` on the `hook` route (a
  preamble frame carrying the session id + token), then writes the raw payload as one
  frame, fire-and-forget. Derives the socket path from `TILLERD_DIR`; no env URL.
  Producer peer of `gate-client`. Dormant in 0.x — runs only when an agent fires hooks.

---

## The backend: orchestrator (`crates/orchestrator`)

The orchestrator is the whole backend, as a runtime-agnostic Rust **library** — not a
process and not one of the shared singletons. Each host embeds it as a Cargo dependency
(the desktop binds its API to Tauri commands + a streaming `Channel`; a future server
binds it to HTTP / WS) and the orchestrator is identical across both
([ADR-0022](../adr/0022-workspace-session-container-above-the-engine.md)).

- **Workspace domain** — projects, sessions, surfaces, launch-spec execution
  ([ADR-0021](../adr/0021-declarative-launch-spec-for-projects-and-sessions.md)), the
  archive lifecycle.
- **Persistence** — rusqlite over `tillerd.db`
  ([ADR-0023](../adr/0023-workspace-data-model-and-two-level-id.md)).
- **Surface runtime** — one PTY proxy per terminal surface, composing
  `daemon-pty-client` over the daemon wire
  ([ADR-0024](../adr/0024-surface-runtime-owns-the-pty-proxy-per-surface.md)).
- **Supervision** — adopt-or-spawns and health-checks the daemon and gate at boot, via
  `process-launch`.
- **API** — a transport-agnostic request/response surface plus outbound streams emitted
  over an `EventSink` trait the host implements.

It exposes two id levels: the product `session_id` stays inside the store and never
leaves; `surface_id` is the only id shared with the daemon (and, in 1.x, the gate)
([ADR-0023](../adr/0023-workspace-data-model-and-two-level-id.md)).

---

## How a terminal surface starts

```
orchestrator                         daemon
     │                                 │
     │ 1. mint surface_id              │
     │ 2. open / attach a PTY ────────▶│  daemon.sock
     │      keyed by surface_id        │  spawns or reattaches the pseudo-terminal
     │                                 │
     │◀── 3. raw bytes ────────────────│
     │      tagged surface_id          │
     │ 4. → EventSink → host → UI      │
```

The surface runtime owns exactly one PTY proxy per terminal surface. `surface_id` is the
daemon's session key; the product `session_id` never crosses the boundary. On host
restart the runtime **reattaches** the live pseudo-terminal by `surface_id` instead of
re-spawning — terminals survive the UI coming and going. If the pseudo-terminal is gone,
the runtime surfaces a typed error
([ADR-0024](../adr/0024-surface-runtime-owns-the-pty-proxy-per-surface.md)).

**Who owns what:**

- **orchestrator** — projects, sessions, surfaces, persistence, and the surface↔PTY
  proxy. Adopt-or-spawns the daemon and gate at boot; cleans up on exit.

> **Agent surfaces (1.0.0).** An agent surface is a PTY surface that *also* registers
> with the gate and subscribes to its hook fan-out. That path — mint `{id, token}`,
> **register before spawn** over the gate's `admin` route (so the agent's first hook
> authenticates), inject `TILLERD_SESSION_TOKEN`, drain hooks into a status / content
> model — is deferred with the agent surface
> ([ADR-0027](../adr/0027-zero-x-is-terminal-only-agent-surface-deferred.md)). The
> register-before-spawn invariant returns with it.

---

## Libraries (shared, no process of their own)

### contracts (`crates/contracts`)

- **What** — shared wire types and the canonical length-prefix frame codec.
- **Why** — one home for `HookEvent`, ids, the route preamble, wire-version constants,
  and the `framing` codec so every Rust service speaks the same wire without re-deriving
  it.
- **Gives you** — `framing::{encode_frame, FrameDecoder, MAX_FRAME_SIZE}` and the
  contract types. Pure: no I/O, no async runtime.

### tillerd-paths (`crates/paths`)

- **What** — the single source of truth for the runtime layout and `TILLERD_*` surface
  ([ADR-0025](../adr/0025-tillerd-paths-as-runtime-layout-source-of-truth.md)).
- **Why** — four crates used to define their own runtime-dir resolver and rebuild the
  same socket / manifest paths; the copies drifted. One owner fixes that.
- **Gives you** — runtime-dir resolution (`$TILLERD_DIR` → `~/.tillerd`), the
  `daemon.sock` / `gate.sock` / `daemon.json` / `tillerd.db` path builders, and
  service-binary resolution (override env → `bin/` or `target/{release,debug}` →
  `~/.local/bin`). Depends only on the standard library — a leaf, no cycles.

### service-host (`crates/service-host`)

- **What** — the "run me as a long-lived process" wrapper. A Rust crate.
- **Why** — gate, daemon, memorya, and mcp-gateway all need the same plumbing. Write it
  once.
- **Gives you** — path resolution, manifest write, SIGTERM/SIGINT handling, in-process
  health, and graceful shutdown (SIGTERM children → 5s grace → SIGKILL → remove manifest,
  no orphans). No health socket — health is an in-process self-check owned by the service.
- **Use it** — implement the `Service` trait, call `run_blocking`:

  ```rust
  trait Service {
      fn config(&self) -> ServiceConfig;            // name, version
      async fn serve(&mut self, ctx: ServeContext); // your main loop
      async fn shutdown(&mut self) {}               // your teardown
      fn health(&self) -> HealthReport { /* default: Serving */ }
  }

  fn main() {
      service_host::run_blocking(MyService::from_env());
  }
  ```

  Your service never handles signals or runtimes itself. The host races your `serve()`
  against the stop signal, logs `health()` at startup and drain, and exits uniformly on
  error.

### process-launch (`crates/process-launch`)

- **What** — the adopt-or-spawn launcher.
- **Why** — never start a second daemon when one is already healthy.
- **Use it** — `adopt_or_spawn(dir, version, timing, probes)` returns `Adopted{pid}` or
  `Spawned{pid}`. It runs the 5-step check above, and on spawn it forks, polls until the
  socket answers, then writes the manifest.

> **service-host vs process-launch** — service-host is _"I am a service"_ (manage my own
> lifecycle from inside). process-launch is _"I start services"_ (manage another process
> from outside). Different jobs, easy to confuse by name.

### daemon-pty-client (`crates/daemon-pty-client`)

- **What** — the PTY session-event wire codec; the sole Rust owner of the daemon's
  binary framing ([ADR-0009](../adr/0009-binary-framing-protocol-for-daemon-ipc.md)).
- **Why** — the orchestrator's surface runtime drives the daemon (open / attach / resize
  / stream) without re-deriving the wire or importing the daemon binary.
- **Gives you** — encode / decode for the daemon's session-event frames. Carries no
  socket — the consumer provides the transport and feeds it bytes.

### gate-client (`crates/gate-client`)

- **What** — the consumer half of the gate's subscribe protocol.
- **Why** — memorya (and 1.0.0's agent surface) need to decode gate frames without
  importing the gate binary.
- **Gives you** — `encode_subscribe_preamble` (the `subscribe` route preamble), the typed
  `SubscriptionFrame` (`Ready`, `Event`, `Error`), and the wire-version handshake. The
  length-prefix framing itself it re-exports from `contracts`. Carries no socket — you
  drive your own connection and feed it bytes.

### redact (`crates/redact`)

- **What** — a pure, deterministic credential + PII scrubber (regex catalog + entropy
  fallback + allowlist).
- **Why** — secrets must not reach logs or any emitted text.
- **Gives you** — `redact(input) -> String`; returns the input unchanged when nothing
  sensitive is detected, and for key/value pairs redacts only the value.

---

## Quick reference

| Name                | Kind    | One line                                                          |
| ------------------- | ------- | ----------------------------------------------------------------- |
| orchestrator        | backend | The whole backend, as an embedded library; client of the singletons. |
| daemon              | service | Owns terminals; streams raw bytes.                                |
| gate                | service | One socket, routes by preamble; auth + normalize + fan-out.       |
| memorya             | service | Captures + searches knowledge locally.                            |
| mcp-gateway         | service | Aggregates MCP servers behind one standard front.                 |
| gate-notify         | client  | Per-hook binary; frames the payload to `gate.sock` (hook route).  |
| contracts           | library | Shared wire types + the canonical frame codec.                    |
| tillerd-paths       | library | Single source of truth for the runtime layout + `TILLERD_*`.      |
| service-host        | library | "Run me as a long-lived process" wrapper.                         |
| process-launch      | library | Adopt-or-spawn a managed backend.                                 |
| daemon-pty-client   | library | The daemon's PTY session-event wire codec.                        |
| gate-client         | library | Decode the gate's subscribe wire.                                 |
| redact              | library | Scrub credentials + PII from emitted text.                        |
| `~/.tillerd/`       | data    | Well-known files; the discovery source of truth.                  |

# Services & Libraries

A map of the moving parts: what each one is, why it exists, and how they find
each other. Start here when the architecture feels hard to follow.

## The big picture

```
  Desktop / Server (composition root)
        │ spawns + registers
        ▼
  ┌───────────┐        ┌───────────┐
  │  daemon   │        │   gate    │
  │  (PTY)    │        │ (ingress) │
  └─────┬─────┘        └─────┬─────┘
        │ raw bytes          │ hook events
        │ (direct)           │ (fan-out)
        ▼                    ▼
       UI            engine · memorya · UI
```

Two long-lived background services: **daemon** (owns terminals) and **gate**
(receives agent hooks). Everything else is libraries they share or apps that
drive them.

The hot path — raw terminal bytes — goes **daemon → UI directly**. It never
touches the gate. The gate only sees lifecycle hooks (session start, tool use,
stop), which it normalizes and fans out to whoever subscribed.

---

## Source of truth: the `~/.tillerd/` directory

There is **no central registry**. Services discover each other by reading
well-known files in `$TILLERD_DIR` (default `~/.tillerd/`). That directory _is_
the source of truth.

| File          | Written by | Read by                                             | Holds                                                                                              |
| ------------- | ---------- | --------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `daemon.json` | daemon     | supervisor                                          | `{pid, version}` — is the daemon already running?                                                  |
| `gate.sock`   | gate       | notify client, orchestrator, engine, memorya, tools | the gate's single front door; every connection opens with a route preamble (length-prefixed frame) |

The gate exposes **one** socket. Each connection's first frame is a **route preamble**
— `{ route, session, token?, wireVersion }` — and the gate demultiplexes on `route`:

| Route       | Opened by         | Credential              | After the preamble                       |
| ----------- | ----------------- | ----------------------- | ---------------------------------------- |
| `hook`      | the notify client | per-session token       | raw hook payload frames, fire-and-forget |
| `tool`      | tools             | per-session token       | tool-call IPC, request/response          |
| `subscribe` | engine, memorya   | none                    | ready → server-push hook-event stream    |
| `admin`     | orchestrator      | admin token (≠ session) | register / deregister a session          |
| `mcp`       | mcp clients       | per-session token       | the stream upgrades to the MCP protocol  |

**How "is it running?" is answered** (adopt-or-spawn):

1. Read the manifest file. Missing → spawn.
2. Is the PID alive? (`signal(0)`) No → spawn.
3. Does the version match exactly? No → spawn a replacement.
4. Is the socket reachable? No → spawn.
5. All yes → **adopt** the running instance.

This is why a desktop restart reuses the live daemon instead of starting a
second one.

---

## Services (long-lived processes)

### daemon (`packages/daemon-pty`)

- **What** — owns every terminal (PTY master fd). One daemon, N sessions.
- **Why** — terminals must survive app restarts and upgrades. A detached daemon
  keeps them alive while the UI comes and goes.
- **Does** — spawn / kill / resize sessions; stream raw bytes to subscribers.
  Knows nothing about hooks, agents, or downstream consumers.
- **Talks** — binary framing (4-byte length + payload) over its control socket.

### gate (`apps/gate`)

- **What** — the single front door for agent-facing traffic. The trust boundary.
  One Unix socket (`gate.sock`), no TCP port; binds nothing else.
- **Why** — one place to authenticate, normalize, and fan out everything an
  agent emits, so no other service has to.
- **Does** — accepts a connection, reads its route preamble, applies the
  route→credential policy, then runs the route. The `hook`/`tool`/`mcp` routes feed
  a fixed middleware pipeline; `subscribe` streams events; `admin` mutates the
  registry; `mcp` upgrades the stream to the MCP protocol.
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
- **Today's limits** — the pipeline is hardcoded in `build_router()`; adding a
  middleware (e.g. redact, firewall) is a code change, not config. Authorization
  is allow-all (`AllowPolicy::All`); per-session or per-tool rules are a reserved
  seam, not yet implemented. The `admin` route is walled by credential, not by a
  separate socket — its protection is the centralized route→credential policy.

### memorya (`apps/memorya`)

- **What** — local knowledge layer. Captures conversation + tool activity, makes
  it searchable.
- **Why** — recall across sessions without sending anything to a server.
- **Does** — subscribes to gate hook events, chunks + embeds them into SQLite,
  serves search over MCP (exact term + vector).

### gate-notify (`apps/gate-notify`)

- **What** — the hook client. A tiny native binary the agent execs on each
  lifecycle event (not a daemon — one shot per hook, then exits).
- **Why** — `curl` can't write length-prefixed frames; a small fast binary keeps
  the agent's hot path cheap (~1-2 ms spawn).
- **Does** — reads the hook payload on stdin, opens `$TILLERD_DIR/gate.sock` on the
  `hook` route (a preamble frame carrying the session id + token), then writes the
  raw payload as one frame, fire-and-forget. Derives the socket path from
  `TILLERD_DIR`; no env URL. Producer peer of `gate-client`.

---

## How a session starts

```
orchestrator                gate                 daemon
     │                       │                    │
     │ 1. mint {id, token}   │                    │
     │ 2. register ─────────▶│ gate.sock (admin)  │
     │                       │  now knows session │
     │ 3. spawn daemon w/ env ───────────────────▶│
     │      TILLERD_DIR                             │
     │      TILLERD_SESSION_ID                       │ 4. runs agent
     │      TILLERD_SESSION_TOKEN                     │    in PTY
     │                       │                     │
     │                       │◀── 5. agent fires hook
     │                       │     gate.sock (hook route)
     │                       │     (auth → normalize → fanout)
```

**The rule:** register before spawn. The gate must know a session exists before
the agent can send its first hook, or that hook fails authentication.

**Who owns what:**

- **supervisor** — the daemon. Decides adopt vs spawn; kills it on exit only if
  _it_ spawned it.
- **orchestrator** — sessions. Mints credentials, registers with the gate,
  injects env vars, deregisters on exit.

> Note: nothing here starts the **gate** — apps assume it is already running and
> degrade quietly if it is not. Starting the gate is external for now.

---

## Libraries (shared, no process of their own)

### contracts-rs (`packages/contracts-rs`)

- **What** — the shared wire types plus the canonical length-prefix frame codec.
- **Why** — one home for `HookEvent`, the ids, the wire-version constants, and the
  `framing` codec, so every Rust service speaks the same wire without re-deriving it.
- **Gives you** — `framing::{encode_frame, FrameDecoder, MAX_FRAME_SIZE}` and the
  contract types. Pure: no I/O, no async runtime (faces add their own socket
  adapters on top).

### service-host (`packages/service-host`)

- **What** — the "run me as a long-lived process" wrapper. A Rust crate.
- **Why** — gate, daemon, and memorya all need the same plumbing. Write it once.
- **Gives you** — path resolution, manifest write, SIGTERM/SIGINT handling,
  in-process health, and graceful shutdown (SIGTERM children → 5s grace → SIGKILL →
  remove manifest, no orphans). No health socket — health is an in-process
  self-check owned by the service.
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

  Your service never handles signals or runtimes itself. The host races your
  `serve()` against the stop signal, logs `health()` at startup and drain, and
  exits uniformly on error.

### process-launch (`packages/process-launch`)

- **What** — the adopt-or-spawn launcher.
- **Why** — never start a second daemon when one is already healthy.
- **Use it** — `adopt_or_spawn(dir, version, timing, probes)` returns
  `Adopted{pid}` or `Spawned{pid}`. It runs the 5-step check above, and on spawn
  it forks, polls until the socket answers, then writes the manifest.

> **service-host vs process-launch** — service-host is _"I am a service"_ (manage
> my own lifecycle from inside). process-launch is _"I start services"_ (manage
> another process from outside). Different jobs, easy to confuse by name.

### gate-client (`packages/gate-client`)

- **What** — the consumer half of the gate's subscribe protocol.
- **Why** — engine and memorya need to decode gate frames without importing the
  gate binary.
- **Gives you** — `encode_subscribe_preamble` (the `subscribe` route preamble), the
  typed `SubscriptionFrame` (`Ready`, `Event`, `Error`), and the wire-version
  handshake. The length-prefix framing itself it re-exports from `contracts-rs`.
  Carries no socket — you drive your own connection and feed it bytes.

---

## Quick reference

| Name           | Kind    | One line                                                         |
| -------------- | ------- | ---------------------------------------------------------------- |
| daemon         | service | Owns terminals; streams raw bytes.                               |
| gate           | service | One socket, routes by preamble; auth + normalize + fan-out.      |
| memorya        | service | Captures + searches knowledge locally.                           |
| gate-notify    | client  | Per-hook binary; frames the payload to `gate.sock` (hook route). |
| orchestrator   | role    | Mints sessions, registers them, injects env.                     |
| supervisor     | role    | Adopts or spawns the daemon; owns its shutdown.                  |
| contracts-rs   | library | Shared wire types + the canonical frame codec.                   |
| service-host   | library | "Run me as a long-lived process" wrapper.                        |
| process-launch | library | Adopt-or-spawn a managed backend.                                |
| gate-client    | library | Decode the gate's subscribe wire.                                |
| `~/.tillerd/`  | data    | Well-known files; the discovery source of truth.                 |

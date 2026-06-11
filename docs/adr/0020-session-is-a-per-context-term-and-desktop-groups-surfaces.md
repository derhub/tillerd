# 0020. "Session" is a per-context term; the desktop session is a container of surfaces

- Status: accepted
- Date: 2026-06-10

## Context

"Session" is used across the system with a different meaning in each subsystem. The desktop is introducing a workspace-level session — a named container the user opens that holds many things at once (a terminal, an agent, and later other kinds). But the daemon, gate, and memorya already use "session" for narrower, unrelated concepts. A single global definition would either trample each subsystem's language or force an artificial unification of four genuinely different domains: terminal multiplexing, trust/ingress, knowledge capture, and workspace UX.

The subsystems already have coherent local meanings, and a shared identifier already threads them: the `contracts` crate carries a `session_id` plus a `correlation_id` so one logical action can be followed across process hops. So the integration substrate exists; what is missing is an explicit statement that these are distinct concepts and how they relate.

One point must not be lost in that statement: the daemon, gate, and memorya are long-lived **singleton services**. One instance of each runs for the whole machine and serves every session; the orchestrator starts them (adopt-or-spawn) and they outlive any session. A desktop session is a *client* of these services — it does not start them and does not contain them.

## Decision

Treat each subsystem as a bounded context that owns its own meaning of "session." Do not unify them into one entity, and do not rename the existing backend "session" terms.

The ubiquitous language per context:

| Context | "session" means | Identity |
| --- | --- | --- |
| Desktop (workspace UX) | a container of surfaces — a named workspace the user creates | workspace/session id |
| Daemon (PTY multiplexing) | one pseudo-terminal plus its process and byte stream | pty session id |
| Gate (trust / ingress) | an authenticated principal and correlation scope used for auth and fan-out | session id + token |
| Memorya (knowledge) | a capture episode — the grouping key for chunks, recall, and consolidation | session id (capture scope) |

The desktop context introduces two new terms:

- **Session** — the container. A named workspace holding many surfaces.
- **Surface** — a leaf within a session, discriminated by **kind** (`terminal`, `agent`, and later others). The container is kind-agnostic.

Relationships (the context map). Two axes, kept separate so the model is not misread as containment:

**Runtime topology** — the shared services run once and serve everyone; a session is a client that references them by id, never a box that holds them:

```
shared, long-lived services (one instance each, serve every session):
  daemon   — registry of all PTYs
  gate     — registry of all authenticated principals
  memorya  — store of all capture scopes
      ▲  referenced by id, never contained
      │
desktop session (a workspace; a client)
  └─ surfaces
       surface{kind: terminal} → one PTY record in the daemon
       surface{kind: agent}    → one record in each of daemon + gate + memorya,
                                  all sharing one correlation_id
```

**Domain mapping** — within each shared service, "session" is a per-agent-run **record**, not a process and not nested in the desktop session:

- A desktop session owns N surfaces.
- A `surface{kind: terminal}` corresponds to one PTY record in the daemon; a bare shell with no hooks has no gate or memorya record.
- A `surface{kind: agent}` corresponds to one PTY record in the daemon and, while it emits hooks, one record in the gate (auth + fan-out) and one in memorya (capture) — all keyed by one `correlation_id`.
- The **agent-run** is the join point: one logical thing each service holds a record for, under that shared id.

Opening a session or a surface within it creates the corresponding records in the already-running services; closing it removes those records. It never starts or stops a service.

Integration rules:

- The shared kernel is the `correlation_id` (with `session_id`) in `contracts`. It is the only identifier shared across contexts; no subsystem imports another's session model.
- Disambiguation happens at one seam — the SDK. The product/desktop term is **session** (the container); the leaf the SDK exposes is **surface**. The backend contexts keep their local "session" unchanged behind the contract. The translation between a `surface{kind: agent}` and the backend agent-run ids lives in the SDK and `contracts` (an anti-corruption boundary).

This ADR records the model and vocabulary only. It does not implement anything; implementation lands in the 0.1 "Session model" roadmap item (per-session layout instead of one global layout, the surface/kind abstraction, and the SDK translation).

## Consequences

- Each subsystem keeps its language; there is no rename churn in the daemon, gate, memorya, or engine.
- Two new concepts — `session` (container) and `surface` (kind-tagged leaf) — are introduced in the desktop and SDK contexts only.
- Surface kind is an extension point: 0.1 ships `terminal` and `agent`; additional kinds (for example browser or sub-agent) are additive later with no change to the container model. This is the seam the next release expands.
- The `correlation_id` is the single cross-context identifier; subsystems integrate by id, never by sharing a session model.
- Service lifecycle is independent of sessions: the daemon, gate, and memorya start once and are shared; a session only creates and removes per-run records inside them. The number of open sessions never changes how many services run.
- The overload only reappears if a backend "session" id surfaces to the product layer as "session." The SDK translation rule exists to prevent exactly that and must be applied consistently at the seam.
- Decision-only; rollback is reverting this file. The decision constrains the 0.1 implementation but ships no code itself.

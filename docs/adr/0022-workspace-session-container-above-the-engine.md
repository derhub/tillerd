# 0022. A workspace library owns the session container; the engine stays the surface runtime

- Status: proposed
- Date: 2026-06-11

## Context

ADR-0020 defined two desktop/SDK concepts — `session` (a container) and `surface`
(a kind-tagged leaf) — but shipped no code. ADR-0021 then defined the launch spec:
a project owns a template, a session is an instance, and launch items produce
surfaces. Something must instantiate templates into grouped, persisted surfaces.

The current code does not have that grouping:

- The engine exposes `AgentSession` (implemented by `AgentSessionProxy`): a handle to
  **one PTY** — one daemon record and, for agents, one hook stream. It carries the
  byte stream, status, content, and lifecycle, with flow control, snapshot replay,
  status mapping, and a send queue. This is a proven per-PTY runtime.
- `Engine` holds a **flat** `Map<id, proxy>` and exposes `start` / `reconnect` /
  `listSessions` / `shutdown`. There is no container grouping multiple PTYs.
- The only "container" today is a thin UI-local `type Session = { id; cwd? }` in the
  desktop shell — no lifecycle, no launch orchestration, no persistence.

So the engine's `AgentSession` already *is* the ADR-0020 surface in all but name. The
missing piece is the **container** plus the launch-and-persistence layer around it —
not a rewrite of the per-PTY runtime.

A full rename of `AgentSession` to `Surface` across the SDK, engine, server, desktop,
and tests was considered and **rejected**: the churn is broad and the runtime is
correct as written. The engine keeps its name.

## Decision

Keep the engine as the surface runtime, unchanged, and add a new layer above it.

### Engine — unchanged

`AgentSession` / `AgentSessionProxy` remain the per-PTY runtime handle. No rename, no
behavior change. It stays web-API-only and pure (ADR-0003).

### Workspace library — new

Introduce a host-agnostic package (name TBD, e.g. `@tillerd/workspace`) above the
engine that owns the workspace domain:

- **`Session`** — the container: `{ id, projectId, title, surfaces, layout }`.
- **`Surface`** — the ADR-0020 leaf: `{ id, kind, … }` wrapping one engine
  `AgentSession`. This is where the product seam presents "surface" in ADR-0020's
  vocabulary, while the engine internally keeps `AgentSession`.
- **Launch-spec execution** (ADR-0021): template → launch items → spawn surfaces via
  the engine, run `pre` / `post`, the worktree step, and placement.
- **Persistence port**: the host injects a SQLite adapter (ports-and-adapters,
  ADR-0003). The library stays pure and web-safe, like the engine.
- **Project domain**: projects, worktrees (under a project, ADR-0021 decision),
  command library, and the archive lifecycle.
- **Container API**: `createSession` / `getSession` / `listSessions` /
  `archiveSession`; `session.addSurface` / `removeSurface`.

### Naming

The product/desktop term is `Session` (the workspace container). The engine's
`AgentSession` is the underlying surface-runtime handle and is not presented to
product code as "the session." The residual overload — engine `AgentSession` versus
workspace `Session` — is bounded: the engine handle is internal to the workspace
library; product and desktop code use `Session` and `Surface`.

### Layering

```
adapter        agent definitions
   ↓
engine         surface runtime — one PTY (AgentSession) each      [unchanged]
   ↓
workspace      Session container + Surface leaves + launch-spec
               execution + persistence port                      [new]
   ↓
host           desktop (Tauri) now / web (server) later —
               injects transport, FS, SQLite, keychain
```

## Consequences

- The proven per-PTY runtime is untouched; no broad rename churn.
- The container, launch execution, and persistence land in one shared library that
  the desktop uses now and the web host reuses later; the engine stays pure.
- A residual naming overload remains (engine `AgentSession` versus workspace
  `Session`). It is the deliberate cost of not renaming, and is contained to the
  engine boundary — product code never names the engine handle "session."
- The persistence port advances the SQLite foundation standard; worktree and keychain
  access are host-injected adapters behind ports.
- The launch spec (ADR-0021) gains a concrete home: the workspace library is what
  reads a template and produces grouped surfaces.
- The decision constrains the 0.x implementation but ships no code itself. Rollback is
  reverting this file.

# Roadmap Plan — Working Notes

Living scratchpad for the 0.x roadmap redesign. Captures the vision, the domain
model, decisions, and open questions from planning. Not the final roadmap — see
[`roadmap.md`](./roadmap.md) once these settle. Decisions here graduate into
ADRs (see [`adr/0021`](./adr/0021-declarative-launch-spec-for-projects-and-sessions.md))
and the roadmap.

> Scope note: **ignore memorya** for the first release. The agent surface still
> reaches the gate (auth / hooks) but no knowledge-capture layer is in scope.

---

## Vision

The first real release delivers a **working workspace**: open the desktop app,
manage projects, start sessions, and drive terminals and agents — with **100%
foundation standards** so future services and features slot in without redesign.

Guiding principle: **minimal library, total schema.** Ship few prebuilt commands,
but a complete launch-spec schema with explicit extension seams.

---

## Domain model

Hierarchy: **project → session → surface**, with a shared **launch spec** and
**command library** underneath.

- **Project** — a named workspace root. Source: `blank` | `local-dir` | `git-repo`
  | `git-worktree`. Name inferred (dir basename / repo name / branch / agent title),
  customizable, renamable. Owns a default launch template.
- **Launch spec** (net-new) — versioned, declarative list of **launch items**. Each
  item: `target` (surface kind), `placement`, `command` (library ref or inline
  `cli+args+env`, default login shell), `pre`/`post` scripts, `autoSpawn` scripts,
  optional `worktree` step (create → cd → run).
- **Command library** — prebuilt (login shell, agent CLI presets) + user-defined
  commands. The extension point for bespoke and future prebuilt workflows.
- **Session** — an instance of a project's template; may diverge. Title inferred
  (agent title | branch | both, user choice) + customizable.
- **Surface** — the running leaf a launch item produces (terminal / agent / diff …).

Detail in [ADR-0021](./adr/0021-declarative-launch-spec-for-projects-and-sessions.md)
(builds on [ADR-0020](./adr/0020-session-is-a-per-context-term-and-desktop-groups-surfaces.md)).

---

## Current state — built vs net-new

Grounded in the code (2026-06-11):

- **Built:** session = container of surfaces (ADR-0020); panel engine (arbitrary
  split / tab / sidebar, `panelTree.ts`); terminal surface (`DesktopTerminalPane`
  → engine → xterm); daemon supervision (`supervisor.rs`, adopt-or-spawn);
  byte bridge (`bridge.rs`); host boot (`useDesktopHost`); agent lifecycle types
  (`IDLE|WORKING|WAITING_INPUT|DONE|crashed`); daemon durability (snapshot /
  replay / resume / stopped-sessions); prefs store (`pref_get/set`).
- **Net-new:** project concept (none in code); launch spec + launch items; command
  library; agent-surface wiring; diff-surface wiring (component exists, unwired);
  gate supervision (no gate supervisor today — only dead-code in `orchestrator.rs`);
  hook-event path; archive model; SQLite persistence.

More findings (2026-06-11):
- **`apps/server`** — Bun web transport: own `server.db` SQLite, `gate-client`,
  `auth`, websocket client-message protocol, uses engine + claude-code adapter.
  Parallel to desktop; dormant. Kept for future web support (decision 11).
- **`adapter-claude-code`** — declarative `AgentDefinition` (launch `claude
  --session-id {id} --dangerously-skip-permissions`, `cliVersionRange >=1.0.0`,
  interrupt `ESC`, binary resolution) + `setup` that installs hooks into the agent's
  `~/.claude/settings.json` (marker `tillerd-notify`, idempotent, strips legacy curl
  hooks). Adapter core is built; hook *parse* → content lives in SDK
  (`hookEventToContent`), status in `SessionStatus`.
- **Agent surface UI does not exist** — only `DesktopTerminalPane`. Agent surface is
  net-new (terminal pane + hook-driven status/content overlay + failure states).
- **Engine is one-proxy-per-id** (`EngineImpl.proxies: Map<sessionId, proxy>`). The
  ADR-0020 container (one desktop session → N surfaces → N daemon records) is
  design-only; multi-surface-per-container is net-new in engine/SDK. **Verify.**

---

## Decisions log

1. **Functionality confirmed** for 0.x: projects, launch templates, sessions,
   surfaces, app shell, foundation standards (full list below).
2. **Project sources:** blank, local-dir, git-repo, git-worktree. Names auto-inferred
   + customizable + renamable. Session titles auto-inferred (agent title | branch |
   both) + customizable.
3. **Launch system is the core.** Project owns a template; session is an instance
   that can diverge. Launch items carry command/args/env, placement, pre/post,
   auto-spawn, worktree step.
4. **Command library:** prebuilt agent CLI presets + user-added custom commands.
   v1 ships minimal prebuilt set; schema complete.
5. **Placement:** v1 = **named regions** (e.g. center / side) mapped onto the panel
   tree. Exact geometry refined **per version**. `placement` field present from day
   one so geometry is additive.
6. **Archive-over-delete:** session delete = archive (recoverable); worktree kept,
   not removed. Hard-delete acts only on archived items, unrecoverable. Foundation
   standard for all destructive actions.
7. **No sandboxing** of pre/post/auto-spawn scripts — local-trust tool; lowest
   priority.
8. **Launch-spec ADR** written → [ADR-0021](./adr/0021-declarative-launch-spec-for-projects-and-sessions.md).
9. **Diff surface** deferred from the first cut (terminal + agent are the v1
   surfaces); diff stays a modeled kind to wire later.
10. **Worktree belongs to a project** (not its own project, not free-floating). A
    git-repo project owns its worktrees; the worktree launch-step registers a
    worktree under the project; sessions reference a project worktree.
11. **Web/server stays dormant, not deleted.** Desktop is the single host now;
    `apps/server` (web transport: own SQLite, gate-client, auth, websockets) is
    parked for future web support. Foundation item is "desktop owns startup; server
    path dormant," not "retire/delete."
12. **Every session has a `project_id`; a built-in "Unfiled" project always exists.**
    Ungrouped sessions belong to Unfiled and render as a flat list. "Move out of a
    project" = reassign to Unfiled. Uniform data model, flexible UX.
13. **Launch-step failure = best-effort, not fail-fast.** A failed item still gets
    its pane, in an error state (failed step + exit code + stderr tail) with
    `[Edit] [Retry] [Logs]`. Other surfaces start normally.
14. **Secrets → OS keychain, no app-rolled crypto.** Secret values live in the OS
    keychain (Keychain / Secret Service / Credential Manager); SQLite holds only a
    reference handle. Launch-spec env splits: `env` (plain) vs `secretRefs`
    (keychain handles). Decrypted value lives only in memory at spawn + in child env
    — accepted boundary; same-user process inspection is not defended (not winnable
    for a local tool).
15. **Launch spec is a versioned JSON Schema with lazy auto-migrate.** Each stored
    template carries `specVersion`; on load, ordered vN→vN+1 pure migrations run
    in-memory and persist the upgraded form. Silent, real-time.
16. **One service contract solidified on `service-host`.** The crate already has a
    `Service` trait used by gate + daemon. Standardize lifecycle (start / ready /
    drain / stop), discovery (socket / manifest convention), health (ADR-0019
    self-check), identity / version. Both services conform; future services inherit.
17. **Observability = structured logs + a log-viewer surface.** Thread
    `correlation_id` across hops; surface logs in the desktop.
18. **One agent (claude-code) 100% = adapter + surface + hook path + preset.**
    Adapter is declarative (launch / binary / hook-install `setup` done); net-new is
    the hook-event path, the agent surface UI (terminal pane + status badge + content
    stream + failure states), the command-library preset, and wiring idempotent
    `setup` into spawn.
19. **Granularity calls:** health indicator is **per-service** (gate / daemon shown
    separately). Settings split **global** (default agent, theme, default command
    library / template) vs **per-project** (launch template, project env, worktree
    config).
20. **Backend inverts to Rust; TS becomes UI + SDK only** (full-now)
    ([ADR-0022](./adr/0022-workspace-session-container-above-the-engine.md)). A Rust
    **`orchestrator` library crate** owns the backend: workspace domain, persistence
    (rusqlite), surface runtime, and agent adapter — composing the existing Rust
    crates (`daemon-pty-client`, `gate-client`, `process-launch`, `contracts-rs`).
    It is **embedded in-process** (Cargo dep), runtime-agnostic, and is the *client*
    of the daemon + gate singletons. Hosts are thin shells binding its
    transport-agnostic API (request/response + `EventSink` streams) to a transport:
    desktop = Tauri commands/events; server (future) = HTTP/WS → remote control. TS
    **engine, adapter-parse, platform-bun, and TS server are retired**; TS keeps
    `ui` (renderer) + `sdk` (typed API client, wire types from `contracts-rs`). Rust
    names fresh: `Session` (container) / `Surface` (leaf) — no TS rename (TS removed).
    *(Supersedes the earlier no-rename / TS-workspace-lib direction.)*
21. **Data model + two-level id** ([ADR-0023](./adr/0023-workspace-data-model-and-two-level-id.md)).
    One product store `~/.tillerd/tillerd.db` (rusqlite, orchestrator-owned); service
    runtime files (daemon.json, snapshots, gate in-memory) excluded. **Two ids:**
    `session_id` (container, product-only, backends never see it) vs `surface_id`
    (= daemon PTY id = gate id = `correlation_id`; today's `TILLERD_SESSION_ID`,
    now per-surface). Tables: project, worktree, launch_template, session, surface,
    command (global library), secret_ref (keychain handle only), setting, meta.
    Launch spec = versioned JSON blob (`spec_json`), not normalized rows. Soft-delete
    via `deleted_at` (archive = `deleted_at IS NOT NULL`; hard-delete = row removal;
    worktree dir kept). "Unfiled" project seeded so `session.project_id` is NOT NULL.
    No pre-v1 data migration.

---

## Functionality list (by layer)

Tags: `[built]` exists · `[wire]` exists but unwired · `[new]` net-new.

**Projects** `[new]`
- Create: blank / local-dir / git-repo / git-worktree
- Name inference + custom + rename
- List / open / remove / persist

**Launch template** `[new]`
- Ordered launch items per project
- Per item: target, placement, command, args, env
- Pre / post scripts; auto-spawn background scripts
- Worktree step (create → cd → run)
- Command library: prebuilt + user-added

**Sessions** `[built]` container, `[new]` CRUD + inference
- Create from template; title inference + custom
- List / switch / rename / archive / resume-after-restart

**Surfaces**
- Terminal `[built]` · agent `[wire]` · diff `[wire, deferred]`
- Placement into split/tab layout `[built]`
- Add / close

**App shell**
- Gate + daemon supervision `[daemon built, gate new]`
- Health / failure-state indicator `[new]`
- Settings / preferences `[built]`
- Window state restore `[new]`

**Foundation standards (the "100%")**
- Launch-spec schema as a versioned contract
- One service contract (lifecycle / discovery / health)
- Single host (desktop owns startup; retire server path)
- Persistence standard (SQLite for durable state)
- Observability (`correlation_id` end-to-end)
- Typed error / failure model surfaced in UI
- Archive-over-delete for destructive actions
- E2E test harness

---

## Parked / future (on the line, version TBD)

- **Containerized execution backend** — dev-container spec, OCI runtimes — behind
  the same launch-item contract (execution-backend extension seam).
- **Web support returns** — `apps/server` revived as the web transport.
- **Prebuilt workflow library** — bespoke workflow sessions, dev-setup presets.
- **Placement geometry** — sizes, nested splits — refined per version.
- **Additional surface kinds** — browser, sub-agent (ADR-0020 extension seam).
- **External secret managers** — beyond OS keychain.

---

## Open questions — still to check

Architecture-critical (resolve before / during data-model design):

- ~~Persistence consolidation + concrete schema~~ — **resolved** (ADR-0023): one
  `tillerd.db` (rusqlite, orchestrator-owned); 9-table schema; service runtime files
  excluded; no pre-v1 migration.
- ~~Engine/SDK multi-surface container~~ — **resolved** (ADR-0022, then superseded):
  backend inverts to a Rust `orchestrator` crate; TS engine retired. Container =
  `Session`, leaf = `Surface`, both in Rust.
- ~~ID / correlation flow for the project layer~~ — **resolved** (ADR-0023):
  two-level id — `session_id` (container, product-only) vs `surface_id` (= daemon /
  gate / `correlation_id`).
- **Agent global-settings mutation** — `setup` writes the user's
  `~/.claude/settings.json`. Policy: when it runs, idempotency, cleanup on uninstall,
  coexistence with the user's own hooks.

Lower / UX-level:

- Worktree archival mechanics — archive dir vs SQLite-mark + leave on disk.
- Command-library identity — ids, versioning, sharing across projects.
- Layout persistence — panel tree per session; restore on resume.
- Cross-platform — keychain backend per OS, worktree availability, signing /
  notarization per OS (macOS + Linux; Windows?).
- E2E harness choice — Tauri-driving framework for the visual test.
- First-run / onboarding — agent binary missing, version out of range, services
  down.
- Which milestones (0.0.x / 0.1.0 / 0.2.0) absorb which layers (roadmap rebuild).

---

## Next steps

1. Resolve open questions above.
2. Rebuild [`roadmap.md`](./roadmap.md) — slot these layers into versioned
   milestones (0.0.x stepping stones → 0.1.0 working workspace → 0.2.0 standardize
   → later: container backend).
3. Spec the launch system via the change workflow when ready to implement.

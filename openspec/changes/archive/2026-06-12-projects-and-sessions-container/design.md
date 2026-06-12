## Context

The orchestrator (ADR-0022) has established a two-level id model (ADR-0023) and owns the surface runtime (ADR-0024). The database schema exists with all nine tables including `project`, `session` (with `layout_json`, `title_source`), `surface` (with `deleted_at`), and the seeded Unfiled row (`ProjectId::UNFILED = "00000000-0000-0000-0000-000000000000"`).

The `SurfaceApi` today creates an unnamed, unowned session for every surface call. There is no way to name, group, list, or resume sessions from the UI. Version 0.0.4 closes that gap by adding complete CRUD for the project → session → surface hierarchy, layout persistence per session, and the archive lifecycle.

The persistence types, store trait skeleton (8 methods), and surface runtime with `SurfaceApi` / `create_terminal_surface` / `create_agent_surface` already exist. The `PanelNode` type system and `usePanelTree` hook exist but are backed only by a single localStorage key. The `SessionSidebar` component renders a flat session list by truncated id and cwd basename.

## Goals / Non-Goals

**Goals:**

- Add orchestrator store operations for project CRUD: create (blank / local-dir / git-repo / git-worktree sources) with name inference, rename, list, archive with cascading soft-delete, hard-delete of archived projects.
- Add orchestrator store operations for session CRUD: create (named, under a project), title inference (agent-title / branch / both, or custom), rename, list (optionally filtered by project), add surface, remove surface, archive with cascade, hard-delete of archived sessions, resume after restart.
- Migrate layout persistence from global localStorage to per-session DB column (`layout_json`).
- Enforce the built-in "Unfiled" project at the operations layer (prevent deletion, ensure all orphan sessions resolve to it).
- Invert the surface-creation flow in `SurfaceApi`: callers supply a `session_id` rather than the API creating an implicit unnamed session per call.
- Expose the new workspace operations through the SDK client so the UI can call them.
- Update the `SessionSidebar` component to render project-grouped sessions with inferred titles and support project/session create and archive actions.

**Non-Goals:**

- Implementing the launch-spec system or command library (ADR-0021 defines the schema; launching from specs is a follow-on change).
- Worktree creation or source detection (projects reference worktrees by id; worktree management is deferred to a later change).
- Hard-delete as a user-facing action (it is an API-level operation for cleanup; the UI does not expose it in 0.0.4).
- Prebuilt project templates or bulk import workflows.

## Decisions

### 1. Store trait expansion: add project, session, and layout methods

**Decision:** Extend the `Store` trait in `crates/orchestrator/src/persistence/sqlite.rs` with methods for project CRUD, session CRUD, and layout operations. Implement all in the `SqliteStore` and add memory-store stubs for tests.

**Rationale:** The store is the single durable backend; all workspace operations flow through it. Grouping project and session methods on the same trait keeps the access pattern consistent with the existing 8 surface-oriented methods and avoids creating a separate trait hierarchy.

**Alternative considered:** Separate traits (ProjectStore, SessionStore, LayoutStore). Rejected: multiple trait objects complicate the host's API surface and testing; one trait with logical groupings is simpler and matches the existing pattern.

### 2. Session creation without a supplied project_id defaults to Unfiled

**Decision:** When a create-session request omits `project_id`, the orchestrator assigns it to the Unfiled project (`"00000000-0000-0000-0000-000000000000"`). This is validated at the store layer, not the API layer.

**Rationale:** The Unfiled project is seeded and immutable (ADR-0023); every session must have a `project_id` NOT NULL. Defaulting to Unfiled makes the data model uniform: no special null-handling logic. Orphan sessions (those created via the old `SurfaceApi` before this change) are ungrouped in the UI but still belong to Unfiled, not null.

**Alternative considered:** Require `project_id` on all create-session calls. Rejected: breaking change for the existing `SurfaceApi` surface creation; defaulting to Unfiled allows a gradual migration path where old code continues to work.

### 3. Title inference strategies: agent-title, branch, both, custom

**Decision:** `title_source` enum has four variants:
- `agent-title`: use the title reported by the agent on session completion; store as empty string until available.
- `branch`: use the current git branch of the session's root path; inferred at creation.
- `both`: concatenate branch + agent title; branch at creation, agent later.
- `custom`: use the caller-supplied `title` verbatim.

**Rationale:** Title inference must be late-bound for agent-title (agents don't report until they finish setup), but early-bound for branch (we can query at session creation). The `both` strategy supports "feat/x - MyAgent" labels. `custom` is the fallback for non-inferred cases.

**Alternative considered:** Single inferred title + a rename operation. Rejected: doesn't support the "both" use case and forces the UI to rename every session that reports an agent title.

### 4. Layout JSON versioning and migration

**Decision:** The `layout_json` blob carries no version field; it is always deserialized as the current `PanelNode` schema. Future schema changes are handled at the UI layer (deserialize with a version-aware deserializer or provide a migration function).

**Rationale:** The layout is a UI concern; the orchestrator never parses it. Embedding a version in the blob adds complexity to the store (it becomes opaque with an inner schema). Versioning the panel schema is a UI library maintenance task (not an orchestrator concern).

**Alternative considered:** `layout_version` column + store-side migration. Rejected: the orchestrator is not the right place for UI schema migrations; keep the blob opaque.

### 5. Surface creation flow inversion: caller supplies session_id

**Decision:** `SurfaceApi::create_terminal_surface` and `create_agent_surface` now require a caller-supplied `session_id`. The surface record is created with that `session_id` FK. The API does not mint a session.

**Rationale:** Sessions own surfaces (ADR-0020); surfaces must be created within an explicit session context. Forcing the caller to supply it makes the container relationship explicit and prevents accidental orphaning.

**Alternative considered:** Keep implicit session creation, add a session-attach API later. Rejected: implicit semantics are hard to unwind; better to require explicit intent from the start.

### 6. Soft-delete vs hard-delete: cascade, timing, and workflow

**Decision:**
- Soft-delete (archive): sets `deleted_at` timestamp. Cascades to children (project → sessions → surfaces).
- Hard-delete: removes row outright. Requires `deleted_at IS NOT NULL` (must be already archived).
- Soft-delete is the user-facing action (UI calls archive-project, archive-session).
- Hard-delete is an internal cleanup tool (exposed in the API but not surfaced in the UI for 0.0.4).

**Rationale:** Archive-over-delete makes destructive actions recoverable by default (ADR-0021 decision #6); users who archive a project can undelete it. Hard-delete is a separate decision (recovery period expired, space reclamation). Separating them keeps the recovery guarantee clear.

**Alternative considered:** Immediate hard-delete with restore-from-trash. Rejected: trash UI is extra complexity; soft-delete in the DB is the minimal change that keeps recovery as a query.

### 7. Unfiled project enforcement

**Decision:** The store layer enforces that `ProjectId::UNFILED` cannot be archived, hard-deleted, or have its name changed. Any such attempt returns a typed error (`ProjectIsUnfiled`). The UI does not expose archive/delete actions on the Unfiled project.

**Rationale:** Unfiled is a container of last resort; it must exist and must be non-empty to guarantee `session.project_id` is always satisfiable. Enforcing at the store layer means all callers (including tests) inherit the guarantee.

**Alternative considered:** Soft constraint in the API layer + validation in the UI. Rejected: the guarantee belongs in the data layer, not the presentation layer.

### 8. Cascading soft-delete is atomic

**Decision:** When a project or session is soft-deleted, the cascade to children happens in a single transaction. The store returns success only if all rows are updated; any conflict aborts the entire operation.

**Rationale:** Partial cascades corrupt the data model (archived project with active sessions) and complicate recovery. Atomicity is the minimal guarantee.

### 9. Resume on startup: reconnect active surfaces

**Decision:** On `Orchestrator::new()` startup, the store is queried for `SELECT * FROM surface WHERE deleted_at IS NULL AND session.deleted_at IS NULL`. For each surface, the runtime calls the existing `SurfaceApi::resume()` path (or the daemon's reconnect API) to re-establish the proxy. Sessions and surfaces are available to clients immediately without requiring a new create-session request.

**Rationale:** The daemon already persists PTY state and can reconnect by `surface_id` (ADR-0023 correlation flow). Surfaces should be available on restart; sessions are queries, not runtime resources, so there is nothing to "resume" for the session itself — only the surfaces.

**Alternative considered:** Lazy reconnect (resume surfaces on first access). Rejected: complicates the client logic; eager reconnect is simpler.

### 10. Name inference for projects: directory basename, repo name, branch

**Decision:**
- `blank`: no name, user must supply.
- `local-dir` / `git-worktree`: use the directory basename.
- `git-repo`: use the repository name (inferred from `.git/config` remote `origin` or repo URL basename).

**Rationale:** Basename is the most common pattern; git-repo name is idiomatic. If inference fails (no git metadata, invalid path), the operation returns an error; the caller must supply an explicit name.

**Alternative considered:** Silent fallback to path string if inference fails. Rejected: silent fallbacks hide errors; fail fast so the caller knows a name is needed.

### 11. Session removal: soft-delete surface, keep PTY alive

**Decision:** `SurfaceApi::remove_surface()` soft-deletes the surface record (sets `deleted_at`) without terminating the PTY. The PTY continues running in the daemon and can be resumed later. Hard-deletion (if/when exposed) terminates the PTY.

**Rationale:** Soft-delete = archival, reversible. Hard-delete = termination, unrecoverable. Matching the project/session pattern makes the cascade model consistent. The spec (surface-runtime.md) requires that hard removal terminate the PTY; soft-delete must not.

**Alternative considered:** Soft-delete terminates, only hard-delete keeps alive. Rejected: breaks the "archive is reversible" guarantee.

### 12. Layout persistence does not version across UI schema changes

**Decision:** The orchestrator stores layout as an opaque JSON blob. The UI layer is responsible for handling schema evolution (e.g., if `PanelNode` structure changes, the UI provides a deserializer that upgrades old formats).

**Rationale:** Versioning layout at the DB layer would require the orchestrator to understand UI schema; keeping it opaque lets the UI own its own schema evolution.

## Open Questions

None identified. All architectural decisions are grounded in ADRs 0020–0023 and the roadmap-plan decisions. Implementation questions (e.g., exact git-metadata detection logic, layout blob size limits) are resolved during code review and test writing.

## Risks / Trade-offs

- **Cascading soft-delete on large projects** — A project with many sessions and surfaces will perform an N-way UPDATE in one transaction. For 0.0.4 scope (single-desktop use case) this is acceptable; future versions may add pagination or async hard-delete cleanup.
- **Session resume on startup assumes daemon is running** — If the daemon is not running or has lost PTY state, reconnect will fail. Mitigation: typed error is surfaced to the user; the session is still queryable and can be resumed after the daemon recovers.
- **Title inference from git branch at creation** — If the branch is later renamed or deleted, the session title becomes stale. Mitigation: the title is user-customizable; no automatic re-inference happens.
- **Layout blob grows unbounded** — A long-running session with many panel mutations accumulates an ever-larger layout JSON. Mitigation: the blob is version-migrable; a future pass can add a cleanup step (e.g., periodically compact the tree).

## Migration Plan

No data migration needed for existing sessions. The `sessions` table already has `layout_json` and `title_source` columns (seeded as NULL / NULL in v1). On first use of the new code, these fields are populated; prior rows remain unchanged and will be treated as "no layout, no inferred title" on restart.

Existing global localStorage layout is discarded (per layout-persistence.md scenario). The first session created under the new API starts with NULL layout and uses the UI's default.


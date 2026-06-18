# 0032. Workspace is the top-level container above project

- Status: proposed
- Date: 2026-06-18

## Context

ADR-0023 made `tillerd.db` the single product store and `project` the root of the
persisted tree (`project -> session -> surface`), with all projects shown in one main
window. ADR-0023's store boundary and two-level `session`/`surface` id model remain in
force and are not revisited here.

Users have no way to group related projects or give a group of work its own window. The
glossary even listed "workspace" as a term to avoid, to keep `project` unambiguous. We now
want a real top tier — a named group of projects that owns its own window — making the
product hierarchy `workspace -> projects -> sessions -> surfaces`.

The data model is frozen at 0.0.6 (additive-only). Two axes had genuine alternatives:
whether a workspace is an organizational grouping or a data-isolation boundary, and
whether it binds to a window or is merely a sidebar filter.

## Decision

Introduce a **Workspace** as the top-level container above `project`.

- **Organizational + window grouping, not an isolation boundary.** One `tillerd.db`
  persists everything; there is no per-workspace path, credential, or daemon isolation.
- **Strict containment.** A project belongs to exactly one workspace, via an additive
  nullable `project.workspace_id` FK. A new `workspace(id, name, sort_order, …)` table is
  added. Both are additive (`migration_vN`), consistent with the frozen-data-model rule.
- **A single, un-deletable Default workspace** with a fixed well-known id (mirroring the
  Unfiled project). The migration backfills every existing project into Default with no
  data loss. Deleting a workspace reassigns its projects to Default rather than deleting
  them.
- **Single main window** with a switcher that selects the active workspace in place; the
  sidebar is scoped to that workspace's projects. A workspace can be detached into its own
  window using the same runtime detach as panel/session/project (the existing `window_open`/
  `window_focus` machinery) — no new host command and no per-workspace geometry.
- **The id model is unchanged.** `workspace_id` is a plain grouping FK; ADR-0023's
  `session`/`surface` ids are untouched. Launch template stays on `project`; no
  workspace-level settings tier is introduced.

## Consequences

- The persisted tree gains a tier without touching the id kernel or any published wire
  contract — the change is purely additive, so older stores migrate forward cleanly.
- The renderer's "all projects" assumption becomes "the active workspace's projects";
  detaching a workspace reuses the existing multi-window plumbing, so no per-workspace
  window geometry is introduced and the single-main-window geometry is untouched.
- Window open/focus stays a host concern via the existing `window_open`/`window_focus`
  host commands (per ADR-0022); the workspace tier adds only a new window intent in the
  renderer, so the orchestrator and SDK stay host-agnostic and no new host command is
  needed.
- A workspace is deliberately not an isolation boundary; if hard isolation (separate
  data/credentials per workspace) is ever needed, it is a separate, larger decision and
  would warrant its own ADR.
- "Workspace" is now a first-class glossary term; `project` is redefined accordingly.

## Why

Projects are the top of the tree today, so every project and session shares one
window and one sidebar. There is no way to group related projects or give a group of
work its own window. A **Workspace** — a named group of projects that owns its own
window — adds the missing top tier: `workspace -> projects -> sessions -> surfaces`.

(0.0.12 "Workspace management" shipped project/session CRUD, not workspaces; this is
the real workspace tier. The roadmap entry is corrected as part of this change.)

## What Changes

- New top-level **Workspace** entity: a named, orderable group of projects. A project
  belongs to **exactly one** workspace (strict containment).
- A single un-deletable **Default** workspace. Every existing project migrates into it
  automatically — zero data loss, opens as one window on first launch.
- **Single main window** with a workspace switcher; selecting a workspace re-scopes the
  sidebar to that workspace's projects in place. A workspace can be **detached into its
  own window** via the same runtime detach as panel/session/project — reusing the existing
  multi-window plumbing. The old "all projects" view becomes "the active workspace's
  projects".
- Workspace is an organizational + window grouping, **not** a data/isolation boundary:
  one `tillerd.db`, no per-workspace path or credential isolation.
- **Glossary**: "Workspace" becomes the term for this entity; the Project definition is
  revised (drops the "named workspace root" wording and the "_Avoid_: workspace" line).
- Schema (additive, `migration_vN`): new `workspace` table + nullable
  `project.workspace_id`. No breaking change; mirrors the `sort_order` migration
  precedent. Launch template stays on project; no workspace-level settings tier.

## Capabilities

### New Capabilities

- `workspace-management`: the Workspace entity and its lifecycle — create, rename,
  delete, reorder; strict project containment; the un-deletable Default workspace;
  the additive migration of existing projects; order persists across restart.
- `workspace-window`: the workspace switcher (selects in place), detaching a workspace
  into its own window via the existing detach machinery, and the sidebar scoping a window
  to its workspace's projects.

### Modified Capabilities

- `project-management`: a project now belongs to exactly one workspace (Default when
  unassigned); moving a project between workspaces is a project operation.

## Impact

- `crates/orchestrator`: `PersistenceStore` gains the `workspace` table, `workspace_id`
  on `project`, the `migration_vN`, and workspace CRUD/reorder/move methods; the
  `Orchestrator` API exposes them.
- `crates/contracts`: workspace types (id, name, sort_order).
- `packages/sdk`: workspace client commands (CRUD/reorder/move). Window detach reuses the
  existing `window_open`/`window_focus` host commands — no new SDK port.
- `apps/ui`: `SessionSidebar` scoping, a workspace switcher, and a `workspace` window
  intent in `windows.ts` for detach.
- Docs: `CONTEXT.md` glossary (Workspace + revised Project); ADR (extends ADR-0023);
  `ROADMAP.md` (relabel 0.0.12 "Project & session management" done; add 0.0.14
  "Workspaces"; bump the existing "UX/UI" milestone to 0.0.15).
- No new crate (modules in existing crates per the layout preference).

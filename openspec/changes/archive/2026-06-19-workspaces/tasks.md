## 1. Persistence + orchestrator (crates/orchestrator, crates/contracts)

- [x] 1.1 Additive `migration_vN`: create `workspace` table, seed Default (fixed id), add
  `project.workspace_id`, backfill all projects to Default. Migration test: pre-workspace
  store -> every project in Default, sessions/surfaces intact.
- [x] 1.2 `PersistenceStore` workspace methods (create/rename/list/reorder/delete) +
  `move_project`; `delete_workspace` reassigns projects to Default; `list_projects` gains
  optional workspace scope and returns `workspace_id`. Rust tests per
  `workspace-management` + `project-management` scenarios (1:1).
- [x] 1.3 `Orchestrator` API wrappers for the above + `workspace`/`Workspace` types in
  `crates/contracts`.

## 2. SDK (packages/sdk)

- [x] 2.1 SDK client methods (create/rename/list/reorder/delete workspace, moveProject,
  scoped listProjects). No new host command — workspace detach reuses the existing
  `window_open`/`window_focus`.

## 3. UI (apps/ui)

- [x] 3.1 `SessionSidebar` lists projects scoped to the active workspace (scoped
  `listProjects`); unit-test the scoping/selection logic (bun:test).
- [x] 3.2 Workspace switcher component (list workspaces, select -> re-scope sidebar in
  place); unit-test list/select logic.
- [x] 3.3 Workspace detach: add a `workspace` `WindowIntent` (+ `workspace-<id>` label,
  `?w=workspace&...` query, `workspace:reattach` event) to `app/lib/windows.ts`, reusing
  `window_open`/`window_focus`; unit-test intent parse + label/query (windows.test.ts).

## 4. Docs

- [x] 4.1 `CONTEXT.md`: add Workspace term; revise Project (drop "named workspace root" +
  "_Avoid_: workspace"). `ROADMAP.md`: relabel 0.0.12 "Project & session management"
  (boxes ticked); add 0.0.14 "Workspaces"; bump existing "UX/UI" milestone 0.0.14 -> 0.0.15.
  (ADR-0032 already added.)

## 5. E2E + verify gate

- [x] 5.1 Desktop e2e (DOM-driveable per testing memory): create workspace, switch
  workspace (sidebar reflects scoped projects), move a project between workspaces, detach a
  workspace into its own window. Detach open/focus asserted via DOM affordance + host
  command contract, not native windows.
- [x] 5.2 Final fix-all gate: `bun run verify` (format/check-types/lint/test) + desktop
  e2e green; `openspec verify` (or manual scenario<->test 1:1) reports no gaps.

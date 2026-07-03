# Tasks: state-model-query-store

## 1. State-model tables + contract fixture (backend)

- [x] 1.1 Add `state_model` module to the entity layer exporting per-entity static
  tables: states, legal transitions, guard rules (`entity`, `action`, `rule`, required
  fields) — data derived from the existing enums and `guard_*` methods, no behavior
  change (spec: state-model-contract / single source).
- [x] 1.2 Rust contract test: serialize the tables (sorted, stable) and assert equality
  with a committed `state-model.contract.json` fixture (command_contract.rs pattern);
  runs in the default verify gate.

## 2. Workspace-activity query + status push (backend)

- [ ] 2.1 `ListWorkspaceActivity` app-layer Query returning
  `{workspace_id, running, failed}` via one GROUP-BY join over persisted surface status;
  transport shim via `transport_query!`; ACL/wire additive (spec: workspace-activity /
  rollup; design D3).
- [ ] 2.2 Rust tests: rollup counts across workspaces (running/failed/idle mix, empty
  workspace) in one round trip; no schema change asserted by migration diff.
- [ ] 2.3 Emit `surface_status_changed {surface_id, session_id, workspace_id, status}`
  on the dispatch spine at every app-layer surface status transition (spawn confirmed,
  exit, error, close); expose over the existing subscription transport (spec:
  workspace-activity / push; design D4).
- [ ] 2.4 Rust test: status transition dispatches exactly one event with the
  post-transition status; event fires after the status write commits.

## 3. Client bindings + state-model mirror (client)

- [ ] 3.1 Regenerate client bindings for the new query + subscription event.
- [ ] 3.2 `apps/ui/app/lib/stateModel.ts`: typed mirror constants + `can(entity, action,
  row)`; TS contract test asserts the mirror matches `state-model.contract.json`,
  including required-field presence on the view types (spec: state-model-contract /
  mirror + drift; design D1/D2).
- [ ] 3.3 Wire enablement: sidebar/context-menu/workspace actions derive
  disabled state from `can()` (Default workspace, Unfiled project, archived entities);
  remove any per-component guard conditionals found (spec: advisory guards).
- [ ] 3.4 UI unit tests: guarded rows render disabled actions; guard rejection from the
  server still surfaces via the mutation-error notification path.

## 4. View pointers (client + settings keys)

- [ ] 4.1 `["view-pointers"]` queryOptions factory reading `view.active-workspace`,
  `view.last-session.<project>`, `sidebar.expanded.<project>` in one round trip
  (ListSettings/ResolveSettings); ApplySetting mutations with optimistic `setQueryData`
  + `meta.invalidates` (spec: view-pointers / cache + optimistic; design D5).
- [ ] 4.2 Migrate `uiStore`: hydrate active-workspace + expanded-projects from the
  view-pointer query; delete their `localStorage` persistence; adopt or delete the dead
  `SIDEBAR_EXPANDED_KEY` constant (spec: client-engine delta).
- [ ] 4.3 Write `view.last-session.<project>` on session open; fire-and-forget failure
  path routed to the standard error channel (spec: view-pointers / fire-and-forget).
- [ ] 4.4 Lifecycle resolution at consumption points: archived/deleted workspace →
  Default + pointer rewrite-once; stale lastSession ignored (spec: view-pointers /
  lifecycle; design D6).
- [ ] 4.5 UI unit tests: pointer restore, stale-pointer fallback, optimistic switch,
  write-failure non-blocking.

## 5. Activity consumption (client)

- [ ] 5.1 `["workspace-activity"]` queryOptions factory; subscription handler feeds the
  crossWindowSync coalescing flush to invalidate `["workspace-activity"]` +
  `["surfaces", session_id]` (spec: workspace-activity / invalidate + coalesce).
- [ ] 5.2 Minimal activity indicator on workspace/session rows reading the rollup
  (running/failed) — full badge styling stays 0.0.20; this proves the read-model
  end-to-end.
- [ ] 5.3 UI unit tests: push event → invalidation (coalesced under burst), badge
  reflects rollup.

## 6. Integration + docs

- [ ] 6.1 Desktop e2e: surface crash/exit updates the activity indicator in a second
  window without user action; view pointers survive relaunch (restore active workspace,
  expanded projects, last session).
- [ ] 6.2 Verify gates green: `cargo nextest run`, bun tests, ast-grep scan (layer
  boundaries hold — entities export data tables only), dynamic-ACL contract test.
- [ ] 6.3 Docs: `docs/tanstack-client-engine.md` conventions updated (view pointers,
  activity, subscription-driven invalidation); CONTEXT.md terms if new ones surfaced;
  ROADMAP 0.0.16 final bullet checked; CHANGELOG entry.

# placement-multi-surface — tasks

## 1. Placement model

- [x] 1.1 Make `placement` an orchestrator-minted UUID, unique per session: mint one per launch
  item when it enters a session spec (template instantiation or spawn); templates carry no
  placement; reject duplicates within a session; lazy-migrate the existing null-placement row to a
  minted placement; drop the `center`/`side` enum (launch-spec + launch-item specs, ADR-0030).
  Test-first: duplicate placement rejected; null row lazy-migrates on open.
  [done: `mint_placements`/`ensure_unique_placements`/`instantiate_for_session` wired into both
  `create_session` paths; null-row lazy-migration in `list_resumable_surfaces` (sqlite+memory).]

## 2. Orchestrator and persistence

- [x] 2.1 Generalize resume to `(session, placement)`: replace `find_session_terminal_surface`
  with a `(session, placement)` resolver over any kind/count that returns absence as a normal
  result; enforce `(session, placement)` uniqueness at surface creation and as a persistence-row
  constraint; on startup reconnect every non-archived surface and expose its placement
  (surface-runtime + session-container specs).
  [done: `find_session_surface_by_placement` (trait + sqlite + memory + SurfaceApi); schema v4
  partial unique index `surface_session_placement`; create_surface maps the constraint to
  `SurfaceConflict`; `surface_create` IPC resolves by placement.]
- [x] 2.2 Persist N placement-keyed surface rows per session; lazy-migrate the null row to a minted
  placement (workspace-persistence spec). Test-first: two-placement session persists two rows and
  resumes both by placement; duplicate-placement write is rejected.
  [done: unique partial index allows N rows/session keyed by placement; lazy-migration in
  `list_resumable_surfaces`; dup-rejection + resolver tests in sqlite + api.]
- [x] 2.3 Spawn/close diverge the launch spec: add-surface appends a launch item, mints a
  placement, creates the surface, and returns the placement; remove-surface is a hard remove --
  drop the item, delete the row, terminate the PTY -- so a closed surface is not resumed
  (session-container spec, ADR-0030). Test-first: add returns a fresh placement; remove drops item
  + kills PTY; restart does not resurrect it.
  [done: `SurfaceApi::spawn_surface`/`remove_surface` + `Store::set_session_spec`; `surface_spawn`
  + `surface_close` IPC commands registered + contract-tested. spawn returns placement (no PTY);
  the pane then calls surface_create to launch. close = drop item + soft-delete row + `runtime.remove`
  (PTY terminate). resume skips soft-deleted rows.]

## 3. Renderer

- [~] 3.1 Bind panel leaves by placement: a leaf resolves its surface via `(session, placement)`
  instead of `<Outlet/>`/self-create and never owns a surface id (ui-panel-model + ui-shell specs).
  Extend the e2e revisit smoke to assert each panel re-attaches its own placement's surface across
  a session switch.
  [done: placement threaded UI->SDK->IPC->resolver; `DesktopTerminalPane` takes a `placement` and
  resolves `(session, placement)`; SDK `create` carries placement; e2e session-revisit green.
  remaining: per-leaf placement binding in the panel tree (still route-`<Outlet/>`, single pane at
  a fixed "main" placement) and the multi-pane revisit assertion.]
- [ ] 3.2 Reconcile the tree against the spec on session open: missing spec placements get a default
  leaf appended to the root, a leaf bound to a placement absent from the spec is dropped, an empty
  (unbound) leaf is kept; an empty spec yields a single empty leaf (layout-persistence +
  ui-panel-model specs).
- [ ] 3.3 Route spawn/close through the orchestrator: the empty-leaf picker calls add-surface and
  binds the acting leaf to the returned placement; closing a surface calls remove-surface (hard
  remove). Splitting stays pure geometry (ui-shell + ui-panel-model specs, ADR-0030).
  [SDK ready: `client.spawn(session)->placement` and `client.close(session, surface)` implemented +
  unit-tested; UI wiring (empty-leaf picker -> spawn, close button -> close) remaining.]
- [ ] 3.4 Lift the sidebar and host-status badge into app-shell chrome; remove the
  `displayMode: 'sidebar'` panel mode and the sidebar/diff entries from the default layout; a fresh
  session opens with an empty spec and a single empty leaf (no auto-open terminal) (ui-shell spec).

## 4. Gate

- [ ] 4.1 Final gate: run `/opsx:verify` and fix all issues, then `bun run verify` and fix all
  issues, then `bun run e2e` and fix all issues.

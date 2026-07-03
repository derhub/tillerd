# Tasks

## 1. Roadmap re-scope (docs)

- [ ] 1.1 In `ROADMAP.md` 0.0.17 section: drop the "Re-sync UX — placement +
  conflict-prompt (Override / Force-merge)" bullet; replace with a one-line note
  that ADR-0044 (state model, sync via server-state cache) and ADR-0039
  (cross-window cache-invalidation broadcast) supersede it — no file merge, no
  conflict prompt.
- [ ] 1.2 Remove "Re-sync" from the end-to-end bullet's flow list
  (create / switch / reload / multi-window).
- [ ] 1.3 Update the 0.0.16 re-scope note (lines ~367–371) so it no longer points
  forward to a live 0.0.17 Re-sync item.
- [ ] 1.4 Check off the 0.0.17 bullets once their work lands.

## 2. Consolidated cross-stack E2E scenario (TDD, red-first)

- [ ] 2.1 Add `tests/desktop-e2e/foundation-integration.test.ts` — one continuous
  journey asserting the four acceptance criteria: create (project + session +
  surface) → switch away and back → reload deep route survives → multi-window
  coherence (write in window A invalidates the matching query in window B).
- [ ] 2.2 Follow the tauri-webdriver + shared-app harness conventions (per project
  testing memory): drive via DOM/route, `uniqueName` per entity, assert behavior
  not attributes, multi-window driven from parent + asserted via host query /
  emitted event (WebDriver sees only the main webview).
- [ ] 2.3 Reuse existing helpers in `tests/desktop-e2e/helpers.ts`; do not
  duplicate per-flow assertions already covered by `project-session`,
  `session-revisit`, `reload-deep-route`, `panel-detach`,
  `view-pointers-restart`, `activity-and-pointers` — this spec proves the axes
  *together* in one flow.
- [ ] 2.4 Red → green: run the new spec against the built app; if the "as one"
  journey exposes an integration blocker, capture it on the decisions page and
  fix the minimum needed (no new product code otherwise).

## 3. Verification

- [ ] 3.1 Run the desktop-e2e suite (`tests/desktop-e2e/run.sh`) — new spec green,
  existing per-flow specs still green.
- [ ] 3.2 Run `bun run verify` (format:check + check-types + lint + test) green.
- [ ] 3.3 `/opsx:verify` drift check — spec acceptance criteria map 1:1 to the new
  e2e assertions.

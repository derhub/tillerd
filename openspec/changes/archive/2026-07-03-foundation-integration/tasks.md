# Tasks

## 1. Roadmap re-scope (docs)

- [x] 1.1 In `ROADMAP.md` 0.0.17 section: drop the "Re-sync UX — placement +
  conflict-prompt (Override / Force-merge)" bullet; replace with a one-line note
  that ADR-0044 (state model, sync via server-state cache) and ADR-0039
  (cross-window cache-invalidation broadcast) supersede it — no file merge, no
  conflict prompt.
- [x] 1.2 Remove "Re-sync" from the end-to-end bullet's flow list
  (create / switch / reload / multi-window).
- [x] 1.3 Update the 0.0.16 re-scope note so it no longer points forward to a live
  0.0.17 Re-sync item.
- [x] 1.4 Check off the 0.0.17 bullets once their work lands.

## 2. Consolidated cross-stack E2E scenario (TDD, red-first)

- [x] 2.1 Add `tests/desktop-e2e/foundation-integration.test.ts` — one continuous
  journey asserting create (project + session + surface) → switch away and back →
  reload deep route survives → multi-window coherence (parent-row reaction).
- [x] 2.2 Follow the tauri-webdriver + shared-app harness conventions (per project
  testing memory): drive via DOM/route, `uniqueName` per entity, assert behavior
  not attributes, multi-window driven from parent (WebDriver sees only the main
  webview). Added the spec to run.sh's own-launch group (it reloads its own app).
- [x] 2.3 Reuse existing helpers in `tests/desktop-e2e/helpers.ts`; do not
  duplicate per-flow assertions already covered by the per-axis specs — this spec
  proves the axes *together* in one flow (surface identity across switch + reload).
- [x] 2.4 Red → green: spec passes against the built app; the "as one" journey
  surfaced no integration blocker.

## 3. Verification

- [x] 3.1 Desktop-e2e suite green — new spec (own-launch batch 4/0) and existing
  per-flow specs (scenario batch 38/0).
- [x] 3.2 `bun run verify` parts green — format:check, check-types, lint, unit test
  suite (16/16), e2e.
- [x] 3.3 `/opsx:verify` drift check — spec acceptance criteria map 1:1 to the new
  e2e assertions.

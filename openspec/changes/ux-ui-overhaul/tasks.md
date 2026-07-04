# Tasks: ux-ui-overhaul

## 1. Foundations

- [x] 1.1 Install missing shadcn base-nova primitives (dialog, dropdown-menu,
  context-menu/menu, select, input, label, switch, checkbox, badge, textarea,
  toggle-group); verify availability against the live registry, fall back to composing
  from `@base-ui/react` where absent
- [x] 1.2 Extend the command manager: `Surface` union gains `activitybar` and
  `statusbar`; handler invocation gains optional argument payload (backward compatible);
  unit tests for arg passing and surface projection
- [x] 1.3 Build the generic `EntityContextMenu` on the shadcn menu primitive fed by
  `contextmenu`-tagged command defs with row-scope `when` + args; delete
  `context-menu-shell.tsx` after migration
- [x] 1.4 Stabilize e2e anchors: inventory `data-testid`/ready-signal selectors used by
  `tests/desktop-e2e/helpers.ts`, define the workbench-era replacements

## 2. Backend additive: swap_placement

- [x] 2.1 TDD `SwapPlacement` command in `crates/orchestrator/src/app/surface/` (atomic
  two-row placement swap, error on unresolved placement; scenario tests mirror the spec)
- [x] 2.2 Register `surface_swap_placement` via `transport_command!`; regenerate
  bindings; contract test passes

## 3. Workbench shell

- [x] 3.1 Build `workbench/` regions: `ActivityBar`, `SidebarContainer` (view registry:
  sessions/search/commands/templates), `BottomPanel` (tab strip), `StatusBar`; restructure
  `RootLayout`/`ShellChrome`; delete `RightDock`, repurpose `BottomDock`
- [x] 3.2 Title bar: retarget toggle toolbar to sidebar + bottom panel + palette; keep
  drag region and native decorations (macOS overlay, Linux system title bar)
- [x] 3.3 Workbench state persistence via settings store (active view, visibilities,
  sizes, active tab) with first-launch defaults
- [x] 3.4 Status bar items: health indicator relocation, workspace/session context,
  notification bell, settings shortcut — all via `statusbar` command projections
- [x] 3.5 Update e2e helpers + existing specs to the new anchors; full e2e green

## 4. Sidebar views: sessions, search, entity actions

- [ ] 4.1 Sessions view polish: status badges (starting/running/failed/idle from
  surface status push), expand/collapse persisted, workspace switcher restyled in view
  header
- [ ] 4.2 Zero state: empty sidebar + center create-project call-to-action
- [ ] 4.3 Project/session/workspace context menus through `EntityContextMenu` with full
  action sets (rename, duplicate, pin/unpin, move, stop-surfaces, open-in-new-window,
  archive, delete)
- [ ] 4.4 Move pickers (project→workspace, session→project) and stop-surfaces
  confirmation dialogs
- [ ] 4.5 Archived sections per entity list with restore + permanent delete
- [ ] 4.6 Pinned-first ordering with pinned indication across workspace/project/session
  lists
- [ ] 4.7 Search view: project + session search across the active workspace, grouped
  results, navigate on activation

## 5. Manager views: commands and templates

- [ ] 5.1 Command library view: list (pinned first, origin badges), create/edit form
  (name, CLI, args, env rows), rename/duplicate/pin/delete with prebuilt guards
- [ ] 5.2 Template manager view: portable library + per-project launch templates
  sections, row actions (pin, delete with prebuilt guard, export, edit/discard)
- [ ] 5.3 Visual launch-spec form editor: ordered item list (command picker from
  library, placement, cwd, env rows), add/remove/reorder, inline validation, serialize
  through apply operations
- [ ] 5.4 Template import/export flows with notification-center feedback

## 6. Settings editor

- [ ] 6.1 `/settings` route rendering the editor shell with section navigation
  (Appearance, Terminal, Keybindings, Profiles, Themes)
- [ ] 6.2 Appearance + Terminal sections (theme, terminal scheme — behavior unchanged,
  shadcn controls); retire the settings popover
- [ ] 6.3 Keybindings section: preset selector + per-command override list relocated
  from the popover
- [ ] 6.4 Profiles section: list/create/activate/rename/duplicate/delete(confirmed)/
  import/export
- [ ] 6.5 Themes section: list/activate/import/export/delete with prebuilt guard

## 7. Bottom panel content: notifications and logs

- [ ] 7.1 Notifications tab: full feed with mark-read, mark-all-read, disregard,
  disregard-all, snooze; unread badge on the status bar bell opens the tab
- [ ] 7.2 Logs tab: reuse `LogViewer` in the bottom panel honoring the service filter;
  `/logs` route retained; health panel logs link opens the tab

## 8. Panel and terminal polish

- [ ] 8.1 Panel title: session name + surface kind + elapsed since spawn (orchestrator
  `spawned_at`); toolbar tooltips on all icon buttons
- [ ] 8.2 Close-surface confirmation dialog with persisted don't-ask-again; hard remove
  (spec item dropped, PTY terminated)
- [ ] 8.3 Empty panel picker listing surface kinds; split buttons create empty leaf →
  picker spawns into the leaf placement
- [ ] 8.4 Panel drag-and-drop swap: header drag, drop-target highlight, cancel outside
  targets, `surface_swap_placement` on drop
- [ ] 8.5 Divider double-click reset to equal split
- [ ] 8.6 Lifecycle motion: opacity-only fades on panel create/destroy and layout
  changes using frozen motion tokens
- [ ] 8.7 Terminal: bundle Geist Mono default font; scheme selection applies live and
  maps terminal token slots; tokenize the crash overlay (remove inline styles)
- [ ] 8.8 Verify terminal copy/paste on macOS and Linux; fix any webview conflicts

## 9. Native menus

- [ ] 9.1 Tauri application menu with accelerators (new project, new session, new
  terminal, close surface, switch session) routed through command ids; platform-correct
  labels; fire with terminal focus

## 10. Visual, accessibility, cross-platform pass

- [ ] 10.1 Icons/spacing/density/typography sweep across all chrome to DESIGN.md tokens;
  one primary moment per view
- [ ] 10.2 ARIA roles/names/states on all interactive chrome; tooltips on every
  icon-only button
- [ ] 10.3 Keyboard navigation: Tab/arrow/Enter/Escape traversal across sidebar tree,
  panel actions, menus, dialogs; visible focus rings
- [ ] 10.4 WCAG AA contrast check of all token pairs in both themes; record results in
  DESIGN.md
- [ ] 10.5 Light-mode sweep of every component (terminal stays dark)

## 11. Performance, E2E, coherence

- [ ] 11.1 Performance pass: chrome responsive under multiple streaming surfaces; no
  unbounded memory growth across session switches
- [ ] 11.2 New e2e specs: panel split + spawn in new leaf; close-surface confirm +
  don't-ask-again persistence; drag-and-drop swap; workbench view switching + state
  restore; settings editor + managers smoke
- [ ] 11.3 Full suite green on macOS and Linux CI
- [ ] 11.4 Update `apps/ui/DESIGN.md` (workbench structure, contrast record), ROADMAP
  0.0.20 checkboxes, CONTEXT.md terms if needed
- [ ] 11.5 Dog-food coherence pass; file 0.1.x follow-ups; absorb any ship blocker as a
  task here

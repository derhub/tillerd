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

- [x] 4.1 Sessions view polish: status badges (starting/running/failed/idle from
  surface status push), expand/collapse persisted, workspace switcher restyled in view
  header
- [x] 4.2 Zero state: empty sidebar + center create-project call-to-action
- [x] 4.3 Project/session/workspace context menus through `EntityContextMenu` with full
  action sets (rename, duplicate, pin/unpin, move, stop-surfaces, open-in-new-window,
  archive, delete)
- [x] 4.4 Move pickers (project→workspace, session→project) and stop-surfaces
  confirmation dialogs
- [x] 4.5 Archived sections per entity list with restore + permanent delete
- [x] 4.6 Pinned-first ordering with pinned indication across workspace/project/session
  lists
- [x] 4.7 Search view: project + session search across the active workspace, grouped
  results, navigate on activation

## 5. Manager views: commands and templates

- [x] 5.1 Command library view: list (pinned first, origin badges), create/edit form
  (name, CLI, args, env rows), rename/duplicate/pin/delete with prebuilt guards
- [x] 5.2 Template manager view: portable library + per-project launch templates
  sections, row actions (pin, delete with prebuilt guard, export, edit/discard)
- [x] 5.3 Visual launch-spec form editor: ordered item list (command picker from
  library, placement, cwd, env rows), add/remove/reorder, inline validation, serialize
  through apply operations
- [x] 5.4 Template import/export flows with notification-center feedback

## 6. Settings editor

- [x] 6.1 `/settings` route rendering the editor shell with section navigation
  (Appearance, Terminal, Keybindings, Profiles, Themes)
- [x] 6.2 Appearance + Terminal sections (theme, terminal scheme — behavior unchanged,
  shadcn controls); retire the settings popover
- [x] 6.3 Keybindings section: preset selector + per-command override list relocated
  from the popover
- [x] 6.4 Profiles section: list/create/activate/rename/duplicate/delete(confirmed)/
  import/export
- [x] 6.5 Themes section: list/activate/import/export/delete with prebuilt guard

## 7. Bottom panel content: notifications and logs

- [x] 7.1 Notifications tab: full feed with mark-read, mark-all-read, disregard,
  disregard-all, snooze; unread badge on the status bar bell opens the tab
- [x] 7.2 Logs tab: reuse `LogViewer` in the bottom panel honoring the service filter;
  `/logs` route retained; health panel logs link opens the tab

## 8. Panel and terminal polish

- [x] 8.1 Panel title: session name + surface kind + elapsed since spawn (orchestrator
  `spawned_at`); toolbar tooltips on all icon buttons
- [x] 8.2 Close-surface confirmation dialog with persisted don't-ask-again; hard remove
  (spec item dropped, PTY terminated)
- [x] 8.3 Empty panel picker listing surface kinds; split buttons create empty leaf →
  picker spawns into the leaf placement
- [x] 8.4 Panel drag-and-drop swap: header drag, drop-target highlight, cancel outside
  targets, `surface_swap_placement` on drop
- [x] 8.5 Divider double-click reset to equal split
- [x] 8.6 Lifecycle motion: opacity-only fades on panel create/destroy and layout
  changes using frozen motion tokens
- [x] 8.7 Terminal: bundle Geist Mono default font; scheme selection applies live and
  maps terminal token slots; tokenize the crash overlay (remove inline styles)
- [x] 8.8 Verify terminal copy/paste on macOS and Linux; fix any webview conflicts

## 9. Native menus

- [x] 9.1 Tauri application menu with accelerators (new project, new session, new
  terminal, close surface, switch session) routed through command ids; platform-correct
  labels; fire with terminal focus

## 10. Visual, accessibility, cross-platform pass

- [x] 10.1 Icons/spacing/density/typography sweep across all chrome to DESIGN.md tokens;
  one primary moment per view
- [x] 10.2 ARIA roles/names/states on all interactive chrome; tooltips on every
  icon-only button
- [x] 10.3 Keyboard navigation: Tab/arrow/Enter/Escape traversal across sidebar tree,
  panel actions, menus, dialogs; visible focus rings
- [x] 10.4 WCAG AA contrast check of all token pairs in both themes; record results in
  DESIGN.md
- [x] 10.5 Light-mode sweep of every component (terminal stays dark)

## 11. Performance, E2E, coherence

- [x] 11.1 Performance pass: chrome responsive under multiple streaming surfaces; no
  unbounded memory growth across session switches
- [x] 11.2 New e2e specs: panel split + spawn in new leaf; close-surface confirm +
  don't-ask-again persistence; drag-and-drop swap; workbench view switching + state
  restore; settings editor + managers smoke
- [ ] 11.3 Full suite green on macOS and Linux CI
- [x] 11.4 Update `apps/ui/DESIGN.md` (workbench structure, contrast record), ROADMAP
  0.0.20 checkboxes, CONTEXT.md terms if needed
- [ ] 11.5 Dog-food coherence pass; file 0.1.x follow-ups; absorb any ship blocker as a
  task here

## 12. Usability pass (commands/templates wired into real flows)

- [x] 12.1 Launch executor resolves item commands (library_ref -> stored cli/args/env,
  inline -> executable/args) into the PTY spawn — restores launch-execution spec
  compliance dropped in the 0.0.15 rewrite; scenario tests per existing spec
- [x] 12.2 `SpawnSurface` accepts an optional command (library ref or inline), appends
  the item to the session's launch spec (ADR-0030 divergence); transport + bindings
- [x] 12.3 New-session flow: template picker (empty / project launch templates /
  library templates); per-project default-template setting honored
- [x] 12.4 Empty panel picker lists Terminal (login shell) + command-library entries
  (pinned first); picking spawns a terminal running that command
- [x] 12.5 Settings: Terminal gains font size, scrollback, cursor style/blink; General
  section (close-confirmation toggle, startup workspace); Project scope section with
  default template
- [x] 12.6 Commands view "Run" row action spawns the command in the active session
- [x] 12.7 Screenshot-driven visual coherence pass, light + dark; before/after captures
- [ ] 12.8 Full verify + affected e2e green after the pass

## 13. Terminal experience baseline (inferred completeness pass)

- [x] 13.1 Find-in-terminal: search overlay per pane (match highlight, next/prev,
  case toggle), palette command + keybinding, xterm search addon
- [x] 13.2 Clickable links in terminal output (URL detection + OSC 8), open via system
- [x] 13.3 Copy/paste hygiene: copy-on-select setting, bracketed-paste verified,
  confirm-before-paste for multi-line clipboard (setting-gated)
- [x] 13.4 Terminal settings: font size, font family, line height, cursor style/blink,
  scrollback size — applied live to mounted terminals via the reactive settings path
- [x] 13.5 Terminal context menu: copy, paste, select all, clear, search selection —
  registry-driven via the contextmenu surface
- [x] 13.6 Bell + attention: xterm bell event surfaces through the notification center;
  unfocused-window bell raises the native banner path
- [x] 13.7 UI zoom setting (webview zoom factor) persisted; window-state persistence
  verified
- [x] 13.8 Drag-and-drop a file onto a terminal inserts its quoted path
- [x] 13.9 General + Project settings sections: close-confirmation toggle, startup
  workspace, per-project default template (folds 12.5 scope)
- [x] 13.10 Follow-ups filed for 0.1.x: shell integration (prompt marking), cwd
  inheritance, session scrollback restore, auto-update — recorded in ROADMAP

# Proposal: ux-ui-overhaul (0.0.20)

## Why

0.0.20 ships the working app, and the current UI does not look or behave like one: the
orchestrator exposes ~127 wire operations but roughly half have no UI equivalent (command
library, templates, profiles, themes, archive/restore, pin, move, search), the right and
bottom docks are literal placeholders, settings is a popover with two native selects, and
context menus are hand-rolled. The milestone turns the existing feature-test shell into a
production-grade, VS Code-like workbench where every app-layer feature has a real UI.

## What Changes

- **BREAKING (UI structure)** Replace the current shell (single sidebar + footer strip +
  placeholder docks) with a VS Code-style workbench: activity bar (icon strip switching
  sidebar views), primary sidebar view container, bottom panel (logs, notifications), full
  status bar (health, workspace/session context, notification bell, settings), title bar
  keeping native window decorations per platform. Right dock placeholder removed.
- New manager UIs over the already-built backend: full settings editor as a panel-area
  surface (appearance, terminal, keybindings incl. presets/overrides, profiles, themes);
  command library and template library as activity-bar sidebar views with inline CRUD.
- Visual launch-spec/template form editor (items: command from library, placement, cwd,
  env rows; add/remove/reorder) — no raw JSON in the normal flow.
- Full app-layer operation coverage in the UI: pin/unpin, archive/restore, duplicate,
  move, stop-surfaces, search for workspace/project/session/command/template, plus
  notification snooze/disregard — reached via context menus, palette, and manager views.
- All roadmap 0.0.20 bullets: interaction polish (sidebar expand/collapse persisted, zero
  state, empty panel picker, pane error overlays), visual polish (icons/spacing/density/
  typography, panel header toolbar + tooltips, panel title with elapsed time, session
  status badges, terminal default font + user-selectable color scheme, all popups/menus on
  shadcn primitives), surface manipulation (split init, panel drag-and-drop swap via new
  orchestrator `swap_placement`, divider resize + double-click reset, terminal copy/paste
  verification, close-surface confirm with don't-ask-again, native menu accelerators),
  motion (fade-only on frozen tokens), light-mode coverage, accessibility (ARIA, keyboard
  nav, WCAG AA), cross-platform title-bar behavior, 60fps/memory performance, E2E coverage.
- Context menus rewired through the command contribution model (`contextmenu` surface) on
  shadcn primitives; new chrome surfaces (activity bar, status bar) consume the same model.
- Backend stays frozen except additive: `swap_placement` command and any additive composed
  read query the sidebar needs.
- `DESIGN.md` updated to document the workbench (tokens unchanged, frozen at 0.0.6).

## Capabilities

### New Capabilities

- `ui-workbench`: workbench chrome — activity bar, sidebar view container, bottom panel,
  status bar; view switching, visibility toggles, persisted layout state.
- `ui-settings-editor`: panel-area settings surface covering appearance, terminal,
  keybindings, profiles, themes; supersedes the settings popover.
- `ui-command-library-view`: command library sidebar view — list, create, rename, edit,
  duplicate, pin, delete; prebuilt commands read-only.
- `ui-template-manager`: template library + per-project launch templates sidebar view with
  the visual launch-spec form editor, import/export.
- `ui-entity-actions`: UI coverage for every exposed app-layer operation (pin/unpin,
  archive/restore, duplicate, move, stop-surfaces, search) via context menus and palette.
- `panel-placement-swap`: drag-and-drop panel leaf swap, backed by an additive
  orchestrator `swap_placement` operation.
- `ui-accessibility`: ARIA roles/labels on chrome, keyboard navigation, WCAG AA contrast.

### Modified Capabilities

- `desktop-title-bar`: toggle toolbar retargets to workbench regions; dock-region and
  visibility-persistence requirements migrate to `ui-workbench`; native decorations per
  platform retained.
- `settings-panel`: popover retired; its requirements migrate to `ui-settings-editor`.
- `ui-session-sidebar`: becomes the Sessions activity-bar view; zero state, status badges,
  expand/collapse persistence.
- `sidebar-context-actions`: context menus move to shadcn primitives driven by the command
  registry `contextmenu` surface; action sets extended to the full operation list.
- `notification-center`: bell moves to the status bar and opens the bottom panel tab;
  mark-read/disregard/snooze actions surfaced.
- `observability-log-viewer`: also hosted as a bottom panel tab (route retained).
- `ui-health-indicators`: health indicator becomes a status bar item.
- `ui-panel-compound`: header toolbar tooltips, title format (session + kind + elapsed),
  close confirmation with don't-ask-again, fade motion, divider reset, empty panel picker.
- `ui-terminal-pane`: shipped default monospace font, user-selectable color scheme wired
  to settings, tokenized error overlay, verified copy/paste.
- `ui-command-manager`: Surface union grows `activitybar`/`statusbar`; `contextmenu`
  surface wired; command invocation gains optional arguments for entity-scoped menus.
- `app-use-case-layer`: additive `swap_placement` command (+ additive composed sidebar
  read if required).

## Impact

- `apps/ui/app/**` — most components touched; shell, sidebar, settings, notifications,
  health, logs, terminal chrome, command defs/registry (breaking UI restructure allowed).
- `apps/ui/app/components/ui/` — new shadcn base-nova primitives installed (dialog,
  dropdown-menu, context-menu, select, input, switch, badge, etc.).
- `crates/orchestrator/src/app/**` + `apps/desktop/src-tauri/src/transport/**` — additive
  `swap_placement` (and possible composed read); wire protocol and ACL otherwise unchanged.
- `apps/desktop/src-tauri/` — native menu accelerators, platform accelerator labels.
- `tests/desktop-e2e/` — new specs (split+spawn, close-confirm persistence, drag swap,
  workbench flows); existing specs updated for the new chrome.
- `apps/ui/DESIGN.md`, `ROADMAP.md` checkboxes, `CONTEXT.md` (workbench terms if needed).
- No dependency changes beyond shadcn component additions; no data-model or wire breaks.

# Design: ux-ui-overhaul (0.0.20)

## Context

The desktop UI is functionally broad but presentation-thin: a single sidebar, a footer
strip, placeholder right/bottom docks, popover settings, hand-rolled context menus, and
no UI at all for the command library, templates, profiles, themes, or the long tail of
entity operations (pin, archive/restore, duplicate, move, stop-surfaces, search). The
backend is complete and frozen (0.0.6 seams; ADR-0036 CQS app layer; 127 wire ops), the
client engine is TanStack Router/Query/Store (ADR-0039/0040), chrome actions flow through
the command contribution model (defs.ts / registry / `useSurfaceCommands`), and the design
tokens are frozen (apps/ui/DESIGN.md — zero radius, 1px borders, 12px root, Geist,
VSCode-2026 palette). Breaking UI restructure is allowed; backend changes must stay
additive.

## Goals / Non-Goals

**Goals:**

- VS Code-like workbench: activity bar, sidebar view container, bottom panel, status bar,
  title bar — production-grade, token-faithful, light+dark.
- A UI equivalent for every exposed app-layer operation.
- Manager surfaces (settings editor, command library, templates with visual spec editor)
  over the existing backend.
- All roadmap 0.0.20 bullets, including a11y, motion, cross-platform title-bar behavior,
  performance, and E2E coverage.

**Non-Goals:**

- Diff surface (0.1.0), nested-split geometry/sizes (0.1.1), agent surface (1.0.0),
  web host, secrets vault, settings-profile cascade semantics changes.
- No new design tokens; no theme redesign — tokens frozen at 0.0.6.
- No toasts — notification center stays the sole feedback channel.
- No wire-protocol, ACL, or data-model breaks.

## Decisions

**D-shell: Workbench regions replace RootLayout's ad-hoc frame.**
`ShellChrome` decomposes into `workbench/` regions: `TitleBar`, `ActivityBar`,
`SidebarContainer`, panel area (`Outlet`), `BottomPanel`, `StatusBar`, composed with the
existing shadcn `resizable` groups. Right dock is deleted. Alternative — incremental
restyle of the current frame — rejected: the placeholder docks and footer cannot express
activity-bar view switching or a tabbed bottom panel without equivalent restructuring.

**D-views: Sidebar views are a static registry, mirroring the command model.**
`VIEW_DEFS` (id, title, icon, component) drives the activity bar: `sessions`, `search`,
`commands`, `templates`. Active view + visibility + sizes persist through the 0.0.9
settings store (global scope), satisfying the roadmap "persisted via 0.0.9" bullets.
Workspace switching stays in the sessions view header (current switcher, restyled), not a
separate activity view — workspaces scope the tree, they are not a parallel view.

**D-managers: Manager surfaces are routes rendered in the panel area, not domain
surfaces.** `/settings` (settings editor), plus sidebar-view-hosted CRUD for commands and
templates; `/logs` route retained. The frozen surface model (panel ↔ placement ↔ surface)
stays terminal-only; a client-side "editor tab" system would duplicate routing for no 0.x
payoff. Opening a manager swaps the panel area via navigation; the session panel tree is
untouched and restored on return (session routes already own that state).

**D-bottom-panel: Bottom panel hosts Logs and Notifications as tabs, reusing the existing
components.** `LogViewer` renders both in the bottom panel and on the `/logs` route
(deep-link + detached window keep working). The notification popover becomes the
Notifications tab (full list, mark-read/disregard/snooze); the status bar bell keeps the
unread badge and toggles the panel. Popover-only alternative rejected: log + notification
triage are resident workflows in a VS Code-shaped app.

**D-statusbar: Status bar consumes the command registry via a new `statusbar` surface.**
Left: workspace/session context, service health (current pill + popover). Right:
notification bell, settings shortcut. New chrome surfaces extend the `Surface` union
(`activitybar`, `statusbar`) exactly as `titlebar` does today — no bespoke per-command UI.

**D-contextmenu: Context menus move to shadcn primitives fed by the registry
`contextmenu` surface.** The registry gains argument-passing (`run(args?)` with the row's
entity id/kind in command context) so one generic `EntityContextMenu` renders defs
filtered by a `when` scope (`menu.scope == "project-row"` etc.). Hand-rolled
`context-menu-shell.tsx` is deleted. This wires the long-tail entity actions (pin,
archive/restore, duplicate, move, stop-surfaces) once, for every row type, and the same
defs surface in the palette.

**D-swap: `swap_placement` is an orchestrator command that atomically swaps two surfaces'
placement bindings within a session.** `SwapPlacement { session_id, placement_a,
placement_b }` — one transaction updating both surface rows; layout tree slots stay put,
surfaces trade slots (ADR-0030 model). Client-only layout_json swap was considered but
the roadmap pins the orchestrator API, and placement-binding swap keeps the launch spec
(placement-addressed) coherent. Additive: new app command + transport macro registration.

**D-spec-editor: The launch-spec/template editor is a typed form over the versioned spec
JSON.** Item list (command picked from the library, placement, cwd, env rows;
add/remove/reorder) serializing to `apply_launch_spec` / `apply_template_spec`. Raw JSON
is not exposed in the normal flow. Spec-version handling stays server-side (lazy
migration, ADR-0021); the client edits only the current version shape.

**D-shadcn: Missing primitives installed from the base-nova registry (`@base-ui/react`),
not hand-built:** dialog, dropdown-menu, context-menu (menu), select, input, label,
switch, checkbox, badge, textarea, toggle-group. Availability verified against the live
registry at install time; any gap falls back to composing from `@base-ui/react` parts
under the same `components/ui/` contract.

**D-terminal-font: Ship Geist Mono (`@fontsource-variable/geist-mono`) as the default
terminal font**, matching the shell's Geist; color scheme selection stays in settings
(0.0.9) and maps onto the `terminal-*` token slots per scheme.

**D-a11y: Accessibility rides the primitives plus explicit chrome contracts.** base-ui
primitives carry dialog/menu semantics; chrome adds ARIA roles/labels (toolbar, tree,
tablist, status), a documented Tab/Enter/Escape traversal across sidebar → panel actions →
dialogs, and a WCAG AA contrast check of all token pairs recorded in DESIGN.md. Terminal
canvas exempt by roadmap.

**D-menus: Native Tauri menu accelerators for the minimal set** (new project, new
session, new terminal, close surface, switch session) with platform-correct labels,
routed through the same command ids so menu, palette, and keybindings stay one system.

## Risks / Trade-offs

- [Milestone-sized diff in one PR] → phased tasks, each phase leaves `bun run verify`
  green; e2e updated in the same phase that breaks its selectors.
- [E2E churn from DOM restructure] → migrate `data-testid` anchors first and keep
  `helpers.ts` ready-signals stable (a removed badge string can be a load-bearing ready
  signal); re-run the full suite per phase.
- [base-nova registry may lack a primitive (e.g. context-menu)] → verified at install
  time against the live registry; fallback is composing from `@base-ui/react` under the
  same component contract.
- [Drag-and-drop is hard to drive in tauri-webdriver] → dispatch real MouseEvents per
  the testing memory; assert placement swap via `data-surface-id`/`data-placement`
  outcomes, not drag visuals.
- [happy-dom has no layout] → cmdk filtering, virtualized lists, and resize behavior are
  e2e-only; unit tests cover open/list/run/dismiss.
- [Registry arg-passing is an API change to the command model] → additive: existing
  no-arg handlers keep working; only `contextmenu`-surfaced defs use args.
- [Bottom panel + sidebar both resizable → layout thrash risk] → sizes persisted,
  clamped min/max, fade-only motion, no animated layout dimensions.

## Migration Plan

Single branch, single PR. Clean UI cutover (breaking allowed); backend additive only.
Rollback = revert the PR; no data or wire migration involved. ROADMAP checkboxes and
DESIGN.md documentation updated in the same change.

## Open Questions

- None blocking. Non-blocking picks (icon choices, exact status bar item order, search
  view result grouping) are logged as decisions during implementation and reviewed at the
  decision gate.

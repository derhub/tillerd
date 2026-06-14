## Why

A session's surfaces are trapped in one window. A user watching a long terminal run cannot
park it on a second monitor while working elsewhere, and a project cannot be opened in its own
window. The orchestrator `EventSink` already supports multiple concurrent subscribers and all
Tauri windows share one backend process, so multi-window is a desktop-host concern only — no
orchestrator-seam change (ADR-0030, ADR-0020).

## What Changes

- Panel detach — a panel header "detach" affordance, shown only on a panel with a live surface,
  tears that panel into a new child window rendering the same surface. The parent panel becomes
  a greyed placeholder with a "Focus" button that raises the child.
- Project in new window — a right-click context menu on a sidebar project row opens that project
  in a child window; the parent sidebar row shows a pending-detach indicator that focuses the
  child on click.
- Re-attach — a child window action returns its panel or project to the parent window and
  focuses the parent; the child closes.
- Window isolation — closing the parent window leaves detached child windows running.
- The detached/child surface re-binds the live PTY byte channel to whichever window mounts the
  pane; the surface stream is unchanged. Detach state is window-runtime only — not persisted to
  `layout_json` (the frozen panel tree, ADR-0030); a relaunch starts in a single window.

## Capabilities

### New Capabilities

- `multi-window`: tear a panel or project into a child Tauri window, the parent placeholder and
  pending-detach indicators, re-attach, focus, and parent/child window isolation.

### Modified Capabilities

- `desktop-shell`: the host gains runtime child-window create / focus / close IPC commands (and
  the Tauri capabilities to allow them); existing single-window boot, menu, and supervision
  behavior is unchanged.

## Impact

- Code: `apps/ui` — panel header (`ui-panel-compound`), `AppShell` render (`ui-shell`), sidebar
  project rows (`ui-session-sidebar`), a child-window route + a cross-window registry/messaging
  module. `apps/desktop/src-tauri` — child-window create/focus/close commands, capabilities.
- APIs: new desktop IPC commands only. No orchestrator API, wire-protocol, or data-model change.
- Dependencies: none new — `@tauri-apps/api/window` (already present) and Tauri core window
  permissions.
- Tests: desktop E2E covering detach -> Focus -> re-attach on macOS and Linux CI.

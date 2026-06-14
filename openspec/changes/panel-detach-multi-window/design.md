## Context

The desktop app runs a single Tauri `main` window hosting the react-router renderer over one
embedded orchestrator backend (`apps/desktop/src-tauri/src/lib.rs`). The panel tree
(`apps/ui/app/lib/panelTree.ts`) is a `PanelNode` recursion of groups and leaves; a leaf's
`content` is `{ type: "terminal", placement }` or `{ type: "empty" }`, serialized to
`layout_json` per session (`usePanelTree.ts`) — the frozen panel-surface seam (ADR-0030).
`AppShell.renderLeaf` renders the panel header (`ui-panel-compound`) and `renderContent` mounts
a `DesktopTerminalPane` keyed by `${sessionId}:${placement}`. The pane invokes `surface_create`
with a per-pane `tauri::ipc::Channel`; the byte channels live in one shared `SurfaceState`
(`surface_host.rs`), and the PTY proxies live in the orchestrator runtime keyed by
`(session, placement)` (`surface/runtime.rs`, `resume` is idempotent on an existing proxy).

All windows of one Tauri app share that single backend and `SurfaceState`. The orchestrator
`EventSink` already fans out to multiple subscribers. So multi-window is renderer-and-host work
only; no orchestrator API, wire-protocol, or data-model change.

## Goals / Non-Goals

**Goals:**

- Detach a live panel into a child window; parent shows a placeholder with a Focus button.
- Open a project in a child window from a sidebar context menu; parent row shows a pending-detach
  indicator.
- Re-attach a panel or project from the child back to the parent, focusing the parent.
- Closing the parent window leaves detached children running.
- No change to the orchestrator seam, `layout_json`, or the surface stream.

**Non-Goals:**

- Persisting detach across relaunch (relaunch opens a single attached window).
- Dragging panels or tabs between windows; child-window geometry beyond OS defaults.
- Multi-window on the web host (desktop only).
- A second concurrent live view of the same surface (detach moves the surface, not clones it).

## Decisions

### A child window is a renderer route against the shared backend

A child window is a `WebviewWindow` loading a dedicated renderer route — `/detached/:sessionId/:placement`
for a panel, the project's session route for a project window. Each window's `DesktopHostProvider`
boots its own orchestrator client and status subscription; the single Rust backend serves all.
Rationale: reuses the existing per-webview boot path and surface rendering untouched; the host
only needs window create/focus/close commands. Alternative — a Rust-side detach registry that
streams panel state — rejected: it would push UI/layout state into the backend and touch the
frozen seam.

### Detach moves the surface by remount; no dual subscription

Detaching unmounts the parent's `DesktopTerminalPane` (its channel drops) and mounts the same
pane in the child, which calls `resume_surface` — the PTY proxy already exists in the runtime, so
it re-binds the byte channel to the child's `Channel` and replays scrollback. The surface is
present in exactly one window at a time, sidestepping any channel-key collision in the shared
`SurfaceState`. Rationale: matches the roadmap "greyed placeholder" model (parent has no live
surface while detached) and needs no change to the channel map keying.

### Detach state is renderer-runtime, never serialized

A cross-window registry module (renderer) maps `placement -> childWindowLabel` and
`projectId -> childWindowLabel`. The parent marks the affected leaf as detached in React state
only — `renderContent` shows the placeholder instead of the terminal — and does NOT write it to
`layout_json`. Window labels encode identity: `detached-<placement>`, `project-<projectId>`.
Rationale: honors the frozen panel-tree seam (ADR-0030); a relaunch deterministically restores a
single attached window.

### Cross-window coordination over Tauri events

Parent and child coordinate with Tauri events, not shared backend state. Re-attach: the child
emits `panel:reattach { sessionId, placement }` (or `project:reattach { projectId }`); the parent
listens, restores the leaf / clears the indicator, focuses itself, and invokes close-window on the
child label. Focus uses the host focus-window command. Rationale: keeps detach orchestration in
the renderer that owns the panel tree; the host exposes only window primitives.

### Shutdown scoped to the last window

The graceful-shutdown hook (`desktop-shell`) moves from per-window-close to last-window-close
(remaining-window count gate). Closing a parent while a child is open closes only that window and
leaves the daemon running. Rationale: required for child-window independence; Tauri already exits
the app on last-window close.

## Risks / Trade-offs

- Live-surface channel re-bind on remount may not be idempotent in `surface_create` -> add or use
  a thin attach path (`resume_surface`) that re-registers the channel without spawning; verify
  scrollback replay in an E2E.
- Brief race if the child mounts before the parent unmounts the same placement -> parent flips to
  placeholder first; child uses the idempotent `resume` (existing proxy returns early).
- Window raise/focus differs across macOS and Linux -> use Tauri `set_focus`; cover both in CI E2E.
- `tauri-webdriver` multi-window E2E support is unproven here -> assert via window count + surface
  presence/`data-surface-id`, not window-manager internals (see project testing notes).
- New Tauri window permissions widen the capability surface -> scope `core:window` allow-create /
  allow-close / allow-set-focus to the app windows in a dedicated capability file.

## Migration Plan

Additive. No schema, migration, or wire-protocol change. New desktop IPC commands plus renderer
routes/components. Rollback is a straight revert; `layout_json` is never written with detach
state, so no persisted state to clean up.

## Open Questions

- Project window: reuse the full `AppShell` scoped to one project, or a trimmed variant? Default:
  reuse `AppShell` navigated to the project's session.
- Re-bind path: extend `surface_create` to re-register an existing surface's channel, or always
  route detached panes through `resume_surface`? Lean `resume_surface`.

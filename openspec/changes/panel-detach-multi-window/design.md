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

### A child window loads at root with an intent query, against the shared backend

A child window is a `WebviewWindow` loading the renderer at the root with an intent query the shell
(`_shell.tsx`) dispatches on: `?w=detached&session=&placement=` renders a single-pane `DetachedWindow`;
`?w=project&project=&session=` renders `AppShell` scoped to that project (via `projectWindowId` +
`initialSessionId`, no route param). A deep route (`/detached/...`) is avoided because the custom
scheme has no SPA fallback for non-root paths. Each window's `DesktopHostProvider` boots its own
orchestrator client; the single Rust backend serves all. The host exposes only `window_open` and
`window_focus` — a child closes itself via the core window API. Alternative — a Rust-side detach
registry that streams panel state — rejected: it would push UI/layout state into the backend and
touch the frozen seam.

### Detach moves the surface by remount; no dual subscription

Detaching unmounts the parent's `DesktopTerminalPane` and mounts the same pane in the child. The
pane's create call goes through the existing `surface_create` revisit path (find-by-placement ->
detach lingering proxy -> resume -> replay scrollback), which re-binds the byte channel to the
mounting window's `Channel`. The child pane passes `detachOnUnmount={false}` so closing it leaves
the live PTY for the parent's revisit path to re-bind — avoiding a channel-remove race. The surface
is present in exactly one window at a time, sidestepping any channel-key collision in the shared
`SurfaceState`. Rationale: matches the "greyed placeholder" model and needs no new host command or
change to the channel map keying.

### Detach state is renderer-runtime, never serialized

A cross-window registry module (renderer) maps `placement -> childWindowLabel` and
`projectId -> childWindowLabel`. The parent marks the affected leaf as detached in React state
only — `renderContent` shows the placeholder instead of the terminal — and does NOT write it to
`layout_json`. Window labels encode identity: `detached-<placement>`, `project-<projectId>`.
Rationale: honors the frozen panel-tree seam (ADR-0030); a relaunch deterministically restores a
single attached window.

### Cross-window coordination over Tauri events

Parent and child coordinate with Tauri events, not shared backend state. Re-attach: the child
emits `panel:reattach { sessionId, placement }` (or `project:reattach { projectId }`) and
self-destroys (via `armReattachOnClose`, so any close path emits first); the parent listens,
restores the leaf / clears the indicator, and focuses itself. Placeholder Focus uses the host
`window_focus` command. Rationale: keeps detach orchestration in the renderer that owns the panel
tree; the host exposes only `window_open` / `window_focus`.

### Shutdown scoped to the last window

The graceful-shutdown hook (`desktop-shell`) moves from per-window-close to last-window-close
(remaining-window count gate). Closing a parent while a child is open closes only that window and
leaves the daemon running. Rationale: required for child-window independence; Tauri already exits
the app on last-window close.

## Risks / Trade-offs

- Channel-remove race when a detached child closes -> the child passes `detachOnUnmount={false}`, so
  only the parent's `surface_create` revisit path re-binds the channel; the child never removes it.
- Window raise/focus differs across macOS and Linux -> use Tauri `set_focus`; cover both in CI E2E.
- `tauri-webdriver` is single-webview and cannot `invoke`/`emit` from `execute` -> E2E asserts only
  the parent's DOM reaction; child-window existence, focus-raise, and the re-attach round-trip drop
  to the command-contract test, the renderer unit tests, and manual verification.
- New Tauri window permissions widen the capability surface -> the existing capability grants
  `core:window` close/destroy/set-focus to the app windows (`main`, `detached-*`, `project-*`) for
  self-close + focus; window creation rides the host `window_open` command (no JS create perm).

## Migration Plan

Additive. No schema, migration, or wire-protocol change. Two desktop IPC commands
(`window_open`/`window_focus`) plus renderer components (`DetachedWindow`, the `?w=` shell branch,
the detach/placeholder/indicator UI). Rollback is a straight revert; `layout_json` is never written
with detach state, so no persisted state to clean up.

## Resolved during implementation

- Project window reuses the full `AppShell` scoped to a project via `projectWindowId` +
  `initialSessionId`, not a trimmed variant.
- Re-bind goes through the existing `surface_create` revisit path (no new attach command); the child
  pane's `detachOnUnmount={false}` keeps the PTY for the parent to re-bind.

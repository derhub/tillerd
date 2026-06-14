## 1. Host window primitives

- [x] 1.1 Add desktop IPC commands `window_open(label, query)` / `window_focus(label)` (`window_host.rs`, generic over `R: tauri::Runtime`); register in `run()` + the contract harness. A child closes itself via the core window API — no close-by-label host command.
- [x] 1.2 Broaden the capability `windows` to cover child windows (`detached-*`, `project-*`) and add `core:window` allow-close/destroy/set-focus for self-close + focus; the window commands are app commands needing no permission string.
- [x] 1.3 Confirm graceful shutdown is bound to `ExitRequested` (last-window close) only — closing a non-last window never drains; document the invariant in `run()`.

## 2. Cross-window plumbing (renderer)

- [x] 2.1 Add `lib/windows.ts`: label helpers (`detached-<placement>`, `project-<projectId>`), `?w=` intent parse, `window_open`/`window_focus` IPC wrappers.
- [x] 2.2 Render a detached child from a `?w=detached&session=&placement=` intent query the shell (`_shell.tsx`) reads (no deep route — the custom scheme has no SPA fallback); `DetachedWindow` shows a single `DesktopTerminalPane`.
- [x] 2.3 Add the cross-window event contract (`panel:reattach`, `project:reattach`) — emit in child, listen in parent.

## 3. Panel detach

- [x] 3.1 Show a detach affordance on the panel header only when `content.type === "terminal"`; on activate, open the child window and mark the leaf detached in renderer-runtime state (not `layout_json`).
- [x] 3.2 Render a greyed placeholder with a "Focus" button (raises the child) where a detached panel's content would be.
- [x] 3.3 Detached pane re-binds the live PTY via the existing `surface_create` revisit path (resume + scrollback replay); child passes `detachOnUnmount={false}` so its close leaves the PTY for the parent to re-bind.

## 4. Project in new window

- [x] 4.1 Add a right-click context menu on a sidebar project row with "Open in new window"; open the `project-<id>` window scoped to that project's first session.
- [x] 4.2 Show a pending-detach indicator on the parent project row that focuses the child on click.

## 5. Re-attach

- [x] 5.1 Re-attach action in child windows (button + native-close via `armReattachOnClose`); emits the re-attach event and self-destroys.
- [x] 5.2 In the parent, on re-attach restore the panel leaf / clear the project indicator and focus the parent (the child closes itself).

## 6. Verification

`tauri-webdriver` drives ONE webview and cannot `invoke`/`emit` from `execute` (bare-specifier
imports fail, `__TAURI__` global off) — it can only click DOM, read DOM, and client-navigate. So
E2E asserts the parent's DOM reaction to each action; child-window existence, focus-raise, and the
re-attach round-trip are not observable and are pushed down to the contract test, the unit tests,
and manual verification.

- [x] 6.1 E2E (multi-window spec 1.2): WHEN a session has an empty leaf THEN no detach affordance is shown (`tests/desktop-e2e/panel-detach.test.ts`).
- [x] 6.2 E2E (spec 1.1, parent side): WHEN a live terminal is detached THEN the panel becomes a greyed placeholder with a Focus button AND the live `data-surface-id` leaves the parent.
- [x] 6.3 E2E (open project in new window, parent side): WHEN "Open in new window" is chosen on a project row THEN the parent row shows a pending-detach indicator.
- [x] 6.4 Renderer unit (`windows.test.ts`): intent parse + label helpers — the cross-window identity contract.
- [x] 6.5 Desktop contract test: `window_open` / `window_focus` dispatch through the live IPC path with their arg shape.
- [x] 6.6 Last-window shutdown invariant: shutdown fires only on `RunEvent::ExitRequested` (documented in `run()`); closing a non-last window never drains. Covers parent-close isolation (spec 8) — not E2E-able (closing the driven window ends the session).
- [x] 6.7 CI: `tests/desktop-e2e/panel-detach.test.ts` green on macOS and Linux (PR #25 — e2e ubuntu + macos both passed).

NOT E2E-able (covered by impl + unit + manual): re-attach round-trip (child-initiated; no emit/invoke from `execute`), child-window existence and focus-raise (single webview), detach→relaunch no-persist (detach state is React-only, never written to `layout_json`, so a relaunch is trivially attached). Verify these by running the app (`/run` or `tests/desktop-e2e/run.sh`).

- [x] 6.8 Local gate green: UI typecheck, UI unit (116), desktop command-contract test, clippy (`-D warnings`), rustfmt, oxfmt, and UI build. Full desktop bundle build + the desktop e2e run in CI (6.7).

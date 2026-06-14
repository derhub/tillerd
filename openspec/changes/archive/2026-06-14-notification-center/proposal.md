## Why

The app has no user-facing feedback channel. Lifecycle signals (a surface starting or
exiting, a surface error, a service going up or down) are emitted today only to per-surface
IPC channels or surfaced as a pulled health snapshot — so a background surface's exit or a
daemon dropping is invisible unless the user happens to be looking at the right pane. The dev
log viewer (0.0.7) shows raw structured logs, not user signal. Roadmap 0.0.10 adds the
notification center: one place that records user-relevant events and raises a native OS
banner when the app is not in focus.

## What Changes

- **In-app notification center** — a bell in the app chrome (the existing bottom-right cluster
  alongside settings + health). Click opens a popover listing recent events with timestamp and
  session/surface context. An unread badge on the bell clears on open.
- **Global notification feed (host-adapter tap)** — the desktop host taps the signals it
  already receives (`TauriSurfaceSink` `on_status`/`on_exit`/`on_error` for every surface, the
  orchestrator `Status` event, and health-snapshot diffs on each status change) and broadcasts
  one global notification event behind a host-agnostic `NotificationSource` port. Additive — no
  orchestrator-core seam is touched.
- **Native OS banners** — `tauri-plugin-notification` raises a system banner (macOS + Linux)
  for background events when the window is unfocused; the banner is dismissable and clicks
  through to the relevant session.
- **Durable history** — persisted in the orchestrator store via an additive `migration_v5()`
  (new `notification` table; existing tables untouched — ADR-0031), survives restarts, bounded by
  prune-on-insert (keep last 500). Reverses the roadmap's "cleared on quit".
- The notification center is the sole user-facing feedback channel — **no toasts** (Sonner is
  not present and stays out).

## Capabilities

### New Capabilities

- `notification-center`: the user-facing event feed — the notification taxonomy and global
  source port, the desktop host-adapter tap that derives notifications from existing lifecycle
  signals, the durable bounded history (orchestrator `notification` table) with unread tracking,
  the bell + popover chrome, and the native OS banner for unfocused background events.

### Modified Capabilities

<!-- None. The feed is built additively at the host-adapter, SDK, and UI layers; the frozen
     0.0.6 orchestrator seams (EventSink, SurfaceEventSink, wire protocol, data model) are
     untouched, so no existing capability's requirements change. -->

## Impact

- **New dependency**: `tauri-plugin-notification` (Rust) + `@tauri-apps/plugin-notification`
  (JS), plus a notification permission entry in the desktop capabilities. Roadmap-mandated.
- **`packages/sdk`**: new notification event taxonomy (types + event/method constants).
- **`crates/orchestrator`**: additive `migration_v5()` + `notification` table; `Store`
  insert/list/prune methods (mirrors the `setting` surface). New ADR-0031.
- **`apps/desktop/src-tauri`**: host-adapter tap over the existing surface + status sinks and
  health snapshots; emits the global notification event; registers the plugin + permission.
- **`apps/ui`**: host-agnostic `NotificationSource` port (desktop adapter now; returns null off
  desktop, server/web deferred — mirrors `LogSource` / `ServiceHealthSource`); in-memory store;
  bell + popover in the bottom-right cluster; native-banner adapter; click-through nav via
  `useNavigate()` (not `<Link>`).
- **Frozen seams**: the data model grows by one additive table (`migration_v5`, ADR-0031); no
  existing table, event, or wire shape changes — the 0.0.6 freeze holds in its additive form.
- **Tests**: `bun:test`+happy-dom for store/diff/chrome logic; desktop e2e for bell→popover→
  click-through nav; the native banner is unit-tested at the plugin boundary (not e2e-reachable).

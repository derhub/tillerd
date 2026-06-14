## Context

The Rust orchestrator owns the backend (ADR-0022/0023); TS is UI + SDK; the desktop host is
Tauri v2. The architecture froze at 0.0.6 — service contract, wire protocol, data model, and
extension seams are additive-only for the rest of 0.x. The lifecycle signals the notification
center needs already exist, but only in forms unsuited to a global, app-wide feed:

- `SurfaceEventSink` (`crates/orchestrator/src/surface/runtime.rs`) emits `on_status`,
  `on_exit`, `on_error` per surface. The desktop adapter `TauriSurfaceSink`
  (`apps/desktop/src-tauri/src/surface_host.rs`) **receives all of them** but routes byte
  output only to the per-surface IPC channel of a mounted pane.
- `EventSink` (`crates/orchestrator/src/boot.rs`) emits the orchestrator `Status`; the desktop
  adapter `TauriEventSink` re-broadcasts it as the global `ORCHESTRATOR_STATUS_EVENT`.
- Service health has **no event** — `ServiceHealthIndicator` pulls a snapshot
  (`service_health` command) and re-reads it on each orchestrator status change.

No global, surface-independent user-signal feed exists, and 0.0.7's log viewer is raw dev logs.

## Goals / Non-Goals

**Goals:**

- A global notification feed that captures lifecycle events from every surface (mounted or not).
- Bell + popover in the existing bottom-right chrome cluster; unread badge; click-through nav.
- Native OS banner for background events when the window is unfocused.
- Durable bounded history that survives restarts (ADR-0031).
- Strictly additive: a new table only; no change to any existing frozen 0.0.6 seam.
- Host-agnostic shape so the future server/web host adds an adapter without touching feature logic.

**Non-Goals:**

- Any orchestrator-core *event* change (new `EventSink`/`SurfaceEventSink` trait method or wire
  field). Persistence is added via an additive new table only (ADR-0031), not a seam rewrite.
- Retry/restart or any control that mutates a service lifecycle (supervision is frozen, read-only).
- Transient toasts. Agent-surface events (0.x is terminal-only).

## Decisions

### D1: Source notifications by tapping the desktop host adapter, not the orchestrator core

The desktop adapters (`TauriSurfaceSink`, `TauriEventSink`) already see every signal. Extend
them — at the **host layer**, not the orchestrator's frozen traits — to also feed a notification
emitter that broadcasts one global Tauri event (`tillerd://notification`). Health-change
notifications are derived by diffing the previous health snapshot against the new one on each
orchestrator-status change (the same trigger the indicator already uses); the diff lives in the
host or is computed UI-side from the snapshot the port already exposes — chosen below.

- *Alternative — extend `SurfaceEventSink`/`EventSink`* (a notification method on the trait):
  rejected. Touches a frozen seam; every later host must reimplement it; the host adapter is the
  correct, additive place.
- *Alternative — UI-only derivation from per-surface channels*: rejected (the asked decision).
  Only mounted surfaces deliver events to the UI, so background surface stop/error would be lost.

### D2: A host-agnostic `NotificationSource` port (mirrors `LogSource` / `ServiceHealthSource`)

`apps/ui/app/lib/transport/notification-source.ts` defines:

```
interface NotificationSource {
  history(): Promise<NotificationEvent[]>;            // durable history on boot (most recent first)
  subscribe(handler: (n: NotificationEvent) => void): Promise<() => void>;  // live feed
}
```

Desktop adapter reads `notifications_list` for history and listens to the `notification://event`
Tauri event for the live feed; `loadNotificationSource()` returns `null` off the desktop host
(server/web deferred), exactly like `loadServiceHealthSource()`. The bell hides when the source
is null. Health-change derivation is done in the desktop host tap (it owns the previous
snapshot), keeping the port's live arm a pure event feed.

### D3: Notification taxonomy in `packages/sdk` (future-ready)

A web-safe `NotificationEvent` (`packages/sdk/src/orchestrator/notification.ts`):
`{ id, category, severity, title?, message, detail?, ts, sessionId?, surfaceId?, actions? }`.
Built future-ready (user ask) without painting into a corner — pre-v1 still allows breaking
changes:

- `category` is an **open union** — the six known kinds (`surface-started`/`-stopped`/`-error`,
  `service-up`/`-down`, `orchestrator-status`) for autocomplete, plus a `string` arm so future
  kinds (agent/diff/workflow) need no schema break. The center renders an unrecognised category
  by its message rather than dropping it.
- `severity` (`info`/`warning`/`error`) drives prominence; `title`/`detail` carry richer content
  beyond the one-line `message`.
- `actions` (`{ label, to }[]`) are extra in-app deep-links rendered as buttons when present —
  beyond the default `sessionId` click-through. 0.0.10 produces none yet; the contract + UI
  support them so future producers add them additively.

Types only — no impl, web-safe (no Buffer/node/Bun), matching the SDK contract rule.

### D4: Reactive UI store hydrated from durable history (with unread tracking)

A small reactive store (provider + hook, the `SettingsProvider` pattern) hydrates from
`list_notifications` on boot, then appends live events from the source. It holds a bounded ring
(`MAX = 200` for display, trim oldest), a derived unread count since last-opened, and a
`markRead()` on open. Pure logic, fully unit-testable without layout.

### D8: Durable persistence in the orchestrator store (ADR-0031)

Notifications persist in the orchestrator store via an additive `migration_v5()` creating a
`notification` table (`id, category, severity, title, message, detail, ts, session_id,
surface_id, actions_json`). The `Store` trait gains `insert_notification` / `list_notifications`
/ `prune_notifications`, mirroring the `setting` surface; `SqliteStore` implements them. The
desktop host tap calls a `notification_host::record(app, store, wire)` that inserts, prunes to
keep the most recent 500, and emits `NOTIFICATION_EVENT`. On boot the UI hydrates via a
`notifications_list` Tauri command behind a host-agnostic `NotificationSource.history()`.

This is the additive form of the 0.0.6 freeze — no existing table changes (ADR-0031) — and it
reverses the roadmap's "cleared on quit". Host-agnostic: the future server host inherits the
same table and `Store` surface.

### D5: Chrome — bell in the bottom-right cluster

A `NotificationIndicator` joins the `fixed bottom-2 right-2` cluster in `AppShell.tsx` beside
`SettingsPanel` + `ServiceHealthIndicator`, using the shared `Popover` and design tokens. The
unread badge is a token-styled count (caps at `9+`).

### D6: Click-through navigation

In-app notification rows that name a session navigate with react-router `<Link to={/session/...}>`.
`<Link>` client-navigates from inside the portaled popover — base-ui portals via React
`createPortal` (router context preserved) and react-router 7.17.0 intercepts internal links
regardless of the opaque `tauri://localhost` origin (verified: the health popover already ships
`<Link>` — see `link-in-portal-navigation`). The native banner has no DOM anchor, so its
activation handler calls `useNavigate()` directly (created in the indicator, which owns router
context).

### D7: Native banner adapter behind a boundary

`apps/ui/app/lib/transport/native-banner.ts` wraps `@tauri-apps/plugin-notification`
(`sendNotification`, permission request/check). It fires only when the window is unfocused
(`getCurrentWindow().isFocused()` or a focus-tracking hook). The store subscribes to the source;
on each new notification it appends to history and, if unfocused, calls the banner adapter. The
Rust side registers `tauri-plugin-notification` and adds a `notification:default` permission to
the desktop capabilities.

## Risks / Trade-offs

- **Health-change false positives at boot** (every service "appears" on first snapshot) → seed
  the previous snapshot from the first read and only emit on subsequent transitions; covered by
  the "unchanged snapshot raises nothing" scenario.
- **Native banner is not e2e-testable** (webdriver can't see native chrome — `testing` memory) →
  unit-test the banner adapter at the plugin boundary (mock the plugin module; assert it is
  called only when unfocused) and e2e only the in-app path.
- **happy-dom has no layout** → keep store/diff/chrome-presence/branch logic in `bun:test`; drive
  list rendering + click-through nav in desktop e2e, asserting the nav **outcome**, not href.
- **Notification permission denied by the OS** → the in-app feed still works; the banner adapter
  no-ops on a denied permission (degrade quietly, consistent with read-only progressive surfacing).
- **Event volume** (a busy surface emitting rapid status changes) → only user-relevant categories
  are surfaced (start/stop/error/health/orchestrator-status), not raw `on_bytes` or every
  IDLE/WORKING flip; the ring bound caps memory regardless.

## Migration Plan

Additive schema migration `migration_v5()` (new `notification` table; existing tables untouched)
applied on next store open — existing DBs run only the new migration. New deps installed in the
worktree at APPLY preflight (`bun install` for the JS plugin, cargo fetch for the Rust plugin).
Rollback = revert the change; the unused table is harmless if left, or dropped on a fresh DB.

## Open Questions

None — sourcing approach, scope, history location, host targets, and nav mechanism are resolved
in the proposal and above.

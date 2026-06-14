## 1. SDK taxonomy

- [x] 1.1 Add the web-safe future-ready `NotificationEvent` type (`id, category, severity, title?,
  message, detail?, ts, sessionId?, surfaceId?, actions?`) with an OPEN `category` union, the
  `NotificationSeverity`/`NotificationAction` types, and the global event-name constant to
  `packages/sdk`; export from the orchestrator index. (D3)

## 2. Durable persistence (orchestrator)

- [x] 2.1 Add `migration_v5()` creating the `notification` table (id, category, severity, title,
  message, detail, ts, session_id, surface_id, actions_json) and append it to `migrations()`. (D8, ADR-0031)
- [x] 2.2 Add `NotificationRecord` + `Store` methods `insert_notification` / `list_notifications(limit)`
  / `prune_notifications(keep)` (trait + `SqliteStore` + `InMemoryStore`), mirroring the `setting` surface. (D8)

## 3. Desktop host-adapter tap

- [x] 3.1 Add `notification_host.rs`: `NotificationWire` (serde camelCase) + `record(app, store, wire)`
  that inserts to the store, prunes to keep-last-500, and emits `notification://event`; plus pure
  builders for each category (unit-tested). (D1, D8)
- [x] 3.2 Extend `TauriSurfaceSink` (`surface_host.rs`) to record surface-started (from
  `surface_create`), surface-stopped (`on_exit`), surface-error (`on_error`), resolving sessionId
  via the store — without changing the `SurfaceEventSink` trait. (D1)
- [x] 3.3 Derive service-up/down + orchestrator-status notifications by diffing the previous health
  snapshot on each status change; seed from the first snapshot so boot emits nothing. (D1)
- [x] 3.4 Register `tauri-plugin-notification` + `notification:default` permission; add JS
  `@tauri-apps/plugin-notification`; add `notifications_list` command. (D7, D2)

## 4. UI port, store, banner

- [x] 4.1 Add the host-agnostic `NotificationSource` port + desktop adapter
  (`apps/ui/app/lib/transport/notification-source.ts`): `history()` (via `notifications_list`) +
  `subscribe()` (via the event); `loadNotificationSource()` returns null off desktop. (D2)
- [x] 4.2 Add the reactive store (provider + hook): hydrate from `history()` on boot, append live
  events, display ring MAX=200 trim-oldest, unread count since last-open, `markRead()`. (D4)
- [x] 4.3 Add the native-banner adapter (`native-banner.ts`) wrapping the plugin; fire only when
  the window is unfocused; no-op on denied permission. (D7)

## 5. Chrome

- [x] 5.1 Add `NotificationIndicator` (bell + unread badge + popover list, most-recent-first,
  empty state) to the bottom-right cluster in `AppShell.tsx`. Each row renders title (or a
  category-label fallback), message, optional detail, a severity indication, timestamp, and any
  `actions` as `<Link>` buttons; an unrecognised category still renders by its message. Session
  rows nav via `<Link to={/session/...}>`; the native-banner activation uses `useNavigate()` (no
  DOM anchor). (D5, D6)

## 6. Verify (final fix-all gate)

- [x] 6.1 Spec scenarios as tests 1:1 (TDD): Rust tests for migration_v5 + insert/list/prune +
  survives-restart; `notification_host` health-diff + builders; `bun:test`+happy-dom for store
  hydrate/append/trim/unread + chrome rich-content/actions/unknown-category + banner-boundary;
  desktop e2e for bell → popover → click-through nav (assert nav outcome). Run `bun run verify`,
  fix until green.
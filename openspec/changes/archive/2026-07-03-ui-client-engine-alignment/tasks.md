# Tasks — ui-client-engine-alignment

## 1. Query-cache reads

- [x] 1.1 `ServiceHealthIndicator`: unit test (red) for rendering from the query cache, then swap
  the effect+`useState` fetch for `useQuery(query("serviceHealth"))` with `enabled` gating and
  `data ?? []` (green). Delete `fetchHealthSnapshot`.

## 2. Non-blocking loader

- [x] 2.1 `routes/session.$id.tsx`: loader returns without awaiting; active-project side effect
  applies when `ensureQueryData` settles. Existing `reload-deep-route` e2e stays green.

## 3. One client-state mechanism

- [x] 3.1 `useDesktopHost`: migrate the module listener `Set` to a `Store<DesktopHostState>` +
  `useStore`; public API unchanged; existing unit tests stay green (adjust only mechanics, not
  assertions).

## 4. Live-tail exception

- [x] 4.1 `LogViewer`: comment the `live` buffer as the high-frequency-stream exception naming the
  spec scenario; add the matching exception note to `docs/tanstack-client-engine.md`.

## 5. Fix-all gate

- [x] 5.1 `bun run verify`, `ast-grep scan` (0 errors), full e2e suite green.

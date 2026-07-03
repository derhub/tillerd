# ui-client-engine-alignment — design

## Context

Four deviations from the client-engine standard survive in `apps/ui`, each pre-dating the pattern
it violates. The cross-window doc/code conflict and ADR statuses are phase 4; this change is code
alignment only.

## Goals / Non-Goals

**Goals:**

- Zero effect+state fetches; zero blocking loader awaits; one client-state mechanism (TanStack
  Store); the live-tail exception explicit instead of implicit.

**Non-Goals:**

- No visual/UX change; render output identical.
- No doc rewrite beyond the live-tail exception note (phase 4 owns the doc reconcile).
- No backend or binding change.

## Decisions

- **`ServiceHealthIndicator` uses plain `useQuery`, not Suspense**: the indicator degrades
  gracefully (renders "starting" aggregate with no data) rather than suspending the shell chrome;
  `enabled: phase !== "web"` gates the desktop-only read, `data ?? []` keeps the prior-snapshot
  behavior (query data persists across an error with `retry: false`).
- **Loader side effect stays, await goes**: `session.$id.tsx`'s loader only needs the project id
  to set active-project scope; it resolves the session via `ensureQueryData` and applies the side
  effect when the promise settles (`.then` in the loader — a plain function, exempt from the
  component async rule), returning immediately. Render-as-you-fetch is preserved; the route's
  Suspense query is already the data path.
- **`useDesktopHost` keeps its public API** (`DesktopHostProvider`, `useDesktopHost()` shape) —
  only the internal mechanism changes to a module `Store<DesktopHostState>` + `useStore`. The
  module-level boot subscription (eager, pre-render) is retained; it writes to the store instead
  of a listener set.
- **LogViewer changes by one comment**: the local `live` buffer is the sanctioned high-frequency
  exception; the comment names the spec scenario so a future reviewer finds the carve-out.

## Risks / Trade-offs

- `useQuery` polls nothing by default where the old effect refetched on phase change — preserved
  by keying the query's `enabled` on phase and relying on mount/invalidation semantics identical
  to `ensureQueryData`'s single-shot; health is also pushed via status events, so staleness risk
  is unchanged.
- Store migration touches boot-path code; covered by existing useDesktopHost/suspense unit tests
  plus the full e2e boot flow.

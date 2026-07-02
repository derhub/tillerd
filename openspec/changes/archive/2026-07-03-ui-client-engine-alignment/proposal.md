# ui-client-engine-alignment

## Why

The freeze audit found four places where `apps/ui` deviates from its own client-engine standard
(ADR-0039, `docs/tanstack-client-engine.md`). Each is exactly the pattern the UI overhaul team
would copy; aligning them now hands the overhaul one correct idiom instead of two competing ones.

## What Changes

- `ServiceHealthIndicator`: replace the effect+`useState` manual fetch with
  `useQuery(query("serviceHealth"))` — the doc's own named anti-example, and a query factory
  already exists.
- `routes/session.$id.tsx`: stop `await`ing `ensureQueryData` in the loader (doc: "last resort");
  switch to the non-blocking `void ensureQueryData` style `index.tsx` already uses, resolving the
  active-project side effect without blocking render.
- `useDesktopHost`: replace the hand-rolled module-level listener `Set` pub/sub with a TanStack
  Store + `useStore`, the client-state mechanism every other shared state already uses.
- `LogViewer` live tail: keep the local-buffer merge (high-frequency stream, same class as PTY
  bytes — per-record cache patching would thrash) but make it an explicit, documented exception:
  a code comment plus a spec-level carve-out for high-frequency streams.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `client-engine`: the single-sync-axis requirement gains an explicit high-frequency-stream
  exception (local buffer feeding the render, cache invalidation for the durable part).

## Impact

- `apps/ui/app/components/health/ServiceHealthIndicator.tsx`, `routes/session.$id.tsx`,
  `lib/useDesktopHost.tsx`, `components/logs/LogViewer.tsx` (comment only),
  `docs/tanstack-client-engine.md` (live-tail exception note only — the broader doc reconcile is
  phase 4).
- No backend, wire, or binding change.

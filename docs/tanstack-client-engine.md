# TanStack client engine — conventions (Query + Store + Router)

How `apps/ui` uses TanStack. Read before touching renderer data-fetching, client state, or routing.
The libraries are the engine (ROADMAP 0.0.16, ADR-0039); these are the patterns we hold to so the
client code stays easy to read and follow. Use the latest TanStack majors (Query v5, Router v1,
Store v0.11+).

## Server state lives in TanStack Query — never in `useState`

Anything the orchestrator owns (projects, sessions, workspaces, ...) is **server state**: it is read
through Query and never copied into `useState`/`useEffect`. A component that fetches in an effect and
stores the result in `useState` is a bug — replace it with `useQuery`.

## Query Options factories (co-locate key + fn)

Per feature, export a `queryOptions`-based factory that co-locates the query key and the query
function (TkDodo, "The Query Options API"). This gives type-tagged keys (`getQueryData` infers the
type), one home per query, and reuse across `useQuery`/`prefetchQuery`/`setQueryData`.

```ts
import { queryOptions } from "@tanstack/react-query";

export const projectQueries = {
  all: () => ["projects"] as const,
  lists: () => [...projectQueries.all(), "list"] as const,
  list: (workspaceId: string | undefined) =>
    queryOptions({
      queryKey: [...projectQueries.lists(), workspaceId ?? null] as const,
      queryFn: () => client().listProjects(workspaceId ? { workspaceId } : undefined),
    }),
  details: () => [...projectQueries.all(), "detail"] as const,
  detail: (id: string) =>
    queryOptions({
      queryKey: [...projectQueries.details(), id] as const,
      queryFn: () => client().getProject({ id }),
    }),
};

// usage
const { data, isPending, isError } = useQuery(projectQueries.list(workspaceId));
```

Key structure: **most generic -> most specific** (`['projects']` -> `['projects','list',ws]` ->
`['projects','detail',id]`). Render from query status (`isPending`/`isError`/`data`), not from
hand-rolled loading flags.

## Mutations auto-invalidate — globally, not per call site

Invalidation after a successful mutation is **automatic**, configured once on the `QueryClient`'s
`MutationCache` (TkDodo, "Automatic Query Invalidation after Mutations"). A mutation declares exactly
the keys it touches via `meta.invalidates`; the global handler invalidates those and nothing else. A
mutation that declares no keys invalidates nothing — we never blanket-invalidate the whole cache (a
missing declaration is a visible bug, not a silent cache wipe). No repeated `onSuccess` boilerplate,
no `refresh()`, no `useRevalidator`. Every write mutation MUST set `meta.invalidates`.

```ts
// queryClient.ts
new QueryClient({
  mutationCache: new MutationCache({
    onSuccess: (_data, _vars, _ctx, mutation) => {
      const keys = mutation.meta?.invalidates as QueryKey[] | undefined;
      keys?.forEach((queryKey) => queryClient.invalidateQueries({ queryKey }));
    },
  }),
  defaultOptions: { queries: { staleTime: 30_000, retry: false } },
});

// a mutation is then a one-liner — meta says what to refetch
function useRenameProject() {
  return useMutation({
    mutationFn: (a: { id: string; name: string }) => client().renameProject(a),
    meta: { invalidates: [projectQueries.all()] },
  });
}
```

`projectQueries.all()` is `["projects"]`, so invalidating it prefix-matches that entity's lists AND
details. Invalidation is cheap (in-process IPC), so coarse-by-entity is fine.

**Optimistic updates** (optional, for instant feel where latency shows — rename, pin, reorder): add
`onMutate` (cancelQueries + snapshot via `getQueryData` + apply via `setQueryData`, return the
snapshot) and `onError` (roll back from the snapshot). The global `MutationCache` handles the final
invalidate. Don't reach for optimism everywhere — only where the user would otherwise see a flash.

## Cross-window coherence — live invalidation broadcast, focus-refetch disabled

Each window has its **own** `QueryClient`, so a mutation's invalidation only refreshes the window it
ran in. Cross-window coherence is a **live invalidation broadcast over the Tauri event bus**
(`lib/crossWindowSync.ts`): on a successful mutation the global `MutationCache` broadcasts the
declared `meta.invalidates` keys on a `query:invalidate` event; every other window's listener
invalidates the matching queries and skips its own broadcast (no echo). A coalesce/dedupe guard
(~80ms flush, both directions) bounds a mutation burst to one emit + one invalidation pass.
`refetchOnWindowFocus` is **disabled** — the broadcast makes it redundant, and it would cause a
refetch storm when many windows regain focus together. `BroadcastChannel` is unfit here: Tauri
windows are separate OS webview processes.

Other channels stay on their own paths, not Query: terminal **surface bytes** ride a per-surface
`ipc::Channel` (raw stream, not cacheable state); orchestrator **status/health** and **notifications**
have their own typed subscriptions. Only domain entity state (projects/sessions/workspaces/...) is
Query-cached.

## Client (UI) state lives in TanStack Store

State the UI owns — selection, active workspace/project, panel/runtime flags — lives in a typed
`Store`, read with `useStore(store, selector)`. The selector keeps re-renders scoped to the slice a
component reads. The Store holds **derived/client** state only; it is never a second copy of server
data the Query cache owns.

```ts
import { Store, useStore } from "@tanstack/react-store";

export const uiStore = new Store({ activeWorkspaceId: null as string | null });
export const setActiveWorkspace = (id: string | null) =>
  uiStore.setState((s) => ({ ...s, activeWorkspaceId: id }));

// component: subscribe to one slice
const activeWorkspaceId = useStore(uiStore, (s) => s.activeWorkspaceId);
```

Use `Derived` for computed state that depends on other store state. Always pass a selector to
`useStore` — subscribing to the whole store re-renders on every field change.

## Router (TanStack Router)

File-based routing via `@tanstack/router-plugin` (ADR-0040, supersedes ADR-0039's code-based single
root). Routes live in `app/routes/` (`__root.tsx`, `index.tsx`, `session.$id.tsx`, `logs.tsx`); the
plugin generates `app/routeTree.gen.ts` (committed, tooling-ignored). The root route's
`validateSearch` is a passthrough `Record<string,string>` and MUST NOT narrow it — the query string
multiplexes window intent (`?w=`) AND view filters (`?service=`). `RootLayout` (the `__root`
component) derives the `WindowIntent` and dispatches: `detached` -> `DetachedWindow`; otherwise the
shell chrome (providers, sidebar scoped by intent, command center, indicators) wraps an `<Outlet/>`
that the path routes fill. The session id comes from the typed `/session/$id` param (or the
project-window intent), provided via `SessionContext`. Deep routes are reload-safe via Tauri v2's
built-in `index.html` asset fallback (default-on) — no custom desktop handler; a reload-at-deep-route
e2e guards it. Client nav stays `history.pushState` + `popstate`.

## Render-as-you-fetch (loaders kick off, Suspense consumes)

Route-critical data follows render-as-you-fetch (ADR-0039 engine, `ui-route-loaders`), NOT
`useQuery` + a loading flag in the component:

- **Loaders kick off the fetch WITHOUT awaiting.** A data route's `loader` calls
  `context.queryClient.ensureQueryData(<the same queryOptions the component reads>)` and voids the
  promise — the route renders immediately and `useSuspenseQuery` latches onto the in-flight request.
  This is render-as-you-fetch, not a waterfall (the fetch starts in the loader, before the
  component). `await` in a loader is the LAST resort (only when a render must truly block) and is
  called out in review. The router context carries the per-window `queryClient`; loader and
  component MUST use the same `queryOptions` so they share one cache entry.
- **`useSuspenseQuery` is the default for route-critical reads.** `data` is always defined; a
  Suspense boundary shows the pending fallback; `defaultErrorComponent` catches failures. `useQuery`
  stays only for genuinely deferred/optional reads — e.g. the panel `layout` (optional when no
  session is selected) and `DiffPanel` (gated on session run-status, reads the HTTP diff API, not the
  orchestrator client).
- **Readiness lives in the queryFn, not `enabled`/`beforeLoad`.** Read queryFns `await
  whenClientReady()` (`lib/data/client.ts`: resolves the orchestrator client when the host is ready,
  or `null` on the web host) before calling the orchestrator. A suspense read therefore pends through
  BOTH boot and fetch into one fallback — no `enabled` flag, no separate boot gate. `DesktopHostProvider`
  keeps the status subscription and resolves the signal.
- **Compose in the frontend; reads stay unscoped, sliced client-side.** The sidebar reads the project
  and session lists once (unscoped) plus the workspace list, each via `useSuspenseQuery`, then derives
  its scoped slice in memory (`useSidebarData`): project window -> that project's sessions; workspace
  scope -> that workspace's projects; main -> all. Scoping is a pure filter, so switching
  workspace/project is INSTANT — no refetch, no suspense flash. The `__root` loader kicks the reads
  off independently (non-awaited, not an awaited `Promise.all`). Do NOT add a server-side composed
  read to "shape one query": an N+1 orchestrator loop over the same per-entity reads is no faster than
  composing on the client. A real SQL-JOIN read is a deferred option, taken only if profiling shows
  the client compose is too slow (instant-first; cache/optimize only what is provably slow).
- **Suspense boundary placement follows progressive boot.** The sidebar suspends into a LOCAL
  `<Suspense>` with a skeleton fallback so the rest of the chrome stays visible while it pends — we do
  not blank the whole shell via a route `pendingComponent`.
- **Realtime coexists: loaders own the snapshot, streams patch the cache.** The loader kicks off the
  initial snapshot; live signals call `setQueryData` (merge) or `invalidateQueries` (refetch) on the
  same entry (the cross-window broadcast already is this). Terminal PTY bytes stay outside Query;
  orchestrator status is the readiness signal the queryFns await, not a cache entry.
- **High-frequency streams render from a bounded local buffer** (client-engine spec scenario).
  Terminal PTY bytes and the live log tail append to a bounded component-local buffer merged at
  render time -- per-record `setQueryData` would re-render every cache subscriber on every line.
  The feature's durable half (backlog windows, file lists) still resolves through the Query cache
  and revalidates by invalidation. This is the one sanctioned exception to the single-sync-axis
  rule.

Test route-critical reads with `renderWithSuspense` (`lib/test/suspense.tsx`): a QueryClientProvider
wrapped in a Suspense boundary, so a `useSuspenseQuery` throw is caught and the test awaits the
resolved content (`findBy*`/`waitFor`). `defaultPreloadStaleTime: 0` keeps Query the single cache owner.

## Web-safety: lazy-load the Tauri API

The Tauri API is reached only through `loadTauriCore()` (`lib/transport/core.ts`), which
`await import("@tauri-apps/api/...")`. This dynamic import is deliberate — the future web build has
no Tauri and must not eagerly bundle it. Never add a top-level `import` of `@tauri-apps/*` in shared
renderer code.

## One QueryClient per window

`makeQueryClient()` builds a per-window client (caches do not leak across webviews). `staleTime` has
a floor so explicit invalidation (mutations + `changed{id}`) drives freshness, not refetch-on-mount.
```

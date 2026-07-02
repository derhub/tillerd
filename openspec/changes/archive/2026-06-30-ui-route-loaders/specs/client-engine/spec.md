## MODIFIED Requirements

### Requirement: Render-as-you-fetch route data with Suspense

Each data route SHALL kick off its reads in a `loader` via
`context.queryClient.ensureQueryData(<the same queryOptions the component reads>)` WITHOUT awaiting
(render-as-you-fetch); the route SHALL render immediately and route-critical reads SHALL be consumed
with `useSuspenseQuery`, suspending into a pending fallback (a Suspense boundary) until ready.
`await` in a loader SHALL be the last resort. Read query functions SHALL obtain the orchestrator
client by awaiting a client-ready signal (resolving the client when the host is ready, or `null` on
the web host) rather than gating with `enabled`, so a suspense read pends through both boot and fetch
into one fallback. Composite views (e.g. the sidebar) SHALL be composed in the frontend from unscoped
entity reads sliced client-side, NOT a server-side composed read; the loader kicks the reads off
independently (non-awaited, not an awaited `Promise.all`). A server-side composed/optimized read is
added only if profiling shows the client compose is too slow. Loader and component MUST use the same
`queryOptions`. Realtime axes are unchanged: streams patch the cache (`setQueryData`/`invalidateQueries`)
or feed their Store; the terminal byte stream stays outside Query.

#### Scenario: A route renders immediately and suspends for its data

- **WHEN** a data route is navigated to
- **THEN** its loader kicks off the fetch without awaiting and the component renders, `useSuspenseQuery`
  suspending into the route `pendingComponent` until the data resolves (no blocking route transition)

#### Scenario: Boot and fetch pend as one through the queryFn

- **WHEN** the app loads before the orchestrator client is ready
- **THEN** the read's queryFn awaits client readiness, the suspense read stays pending, and the
  pending fallback shows -- with no `enabled` flag and no separate boot gate

#### Scenario: A composite view is composed in the frontend, not by a backend read

- **WHEN** a route needs a tree/composite (e.g. the sidebar)
- **THEN** the frontend reads the unscoped entity lists via `useSuspenseQuery` and slices them
  client-side; switching scope is an in-memory filter, not a refetch
- **AND** no server-side composed read is added unless profiling shows the client compose is too slow

#### Scenario: The web host renders without a client

- **WHEN** the client-ready signal resolves `null` (web host)
- **THEN** the route queryFns short-circuit to the web surface and no suspense query hangs

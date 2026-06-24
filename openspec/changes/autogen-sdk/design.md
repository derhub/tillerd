## Context

The desktop IPC layer is a custom Tauri RPC bridge, not HTTP REST: the renderer calls
`transport.invoke(method, args)` and `transport.listen(event)`. Commands are pure CQS values in
`orchestrator::app::*`; the `transport_command!` / `transport_query!` / `transport_create!` macros emit
the `#[tauri::command]` shims (deserialize wire args → `Bus` dispatch → `shared::Error`→string → map to a
`*View` wire DTO), and `collect_transport!` expands to `tauri::generate_handler![...]` over ~120 idents
(~90 macro-generated, ~30 hand-written host/settings/surface/notification shims). The `*View` DTOs already
carry `#[derive(Serialize)] + #[serde(rename_all = "camelCase")]`. `command_contract.rs` asserts every
command's wire shape at runtime.

Above the bridge, three layers restate the same contract by hand: the wire types and the
`invoke(method, args)` mapping in `packages/sdk/src/orchestrator`, and one TanStack hook per operation in
`apps/ui/app/lib/data`. They drift independently; a `*View` field rename is caught only at runtime.

Two boundary facts shape the design:
- `@tillerd/sdk` is **zero-runtime-dependency and transport-agnostic**: its `client.ts` exposes an
  injectable `OrchestratorHostTransport` (invoke/listen); desktop injects Tauri, tests inject a fake. The
  SDK's root/protocol/types are consumed by `packages/logger`, `apps/ui`, and sdk tests.
- `@tillerd/sdk/orchestrator` is consumed **only by `apps/ui`** (~42 files). It is the apps/ui-only outlier.

Because the transport is IPC RPC, **OpenAPI is the wrong IDL** (it models HTTP paths/verbs/status that do
not exist here; the only HTTP in the repo is `apps/mcp-gateway`, a separate MCP protocol). The correct
source of truth is the Rust definitions, exported to TypeScript via **tauri-specta**.

## Goals / Non-Goals

**Goals:**

- Generate the TypeScript bindings (commands, queries, events) from the Rust IPC layer; make them the
  single source of truth for the desktop wire types and the typed invoke client.
- Keep `@tillerd/sdk` zero-dep and transport-agnostic; isolate the Tauri-coupled generated client in a new
  `@tillerd/client-bindings` package.
- Generate the entire TanStack hook surface (queries, mutations, events) with **zero per-entity config**,
  driven by a verb→strategy convention + arg-type inference + a tiny overrides map.
- Keep the wire byte-identical (`command_contract.rs` is the gate) and guard both generated files against
  drift in CI.

**Non-Goals:**

- No HTTP/OpenAPI surface; no remote/non-Tauri consumers.
- No change to CQS core semantics, the `Bus`, or the command set.
- No rework of the cross-window invalidation broadcast (already in `client-engine`).
- No migration of `ipc::Channel` surface I/O off its current transport-resident shape.

## Decisions

> **SUPERSEDED at APPLY (Decision 12): runtime client over generated records.**
> Decisions **4, 8, 9, 11** (build-time emitter producing per-entity option-factory *records* in
> `factory-options.gen.ts`) and the records half of **10** are replaced by a tiny hand-written runtime
> client, `packages/client-bindings/src/client.query.ts`. The records, their emitter
> (`emit-factory-options.ts`), and the runtime spike are deleted. Decisions 1-3, 5-7, and the events/wire
> halves of 8/10 still hold. See Decision 12 below for the runtime-client design and rationale.

**1. tauri-specta over hand-written types, OpenAPI, taurpc, or ts-rs.**
tauri-specta types Tauri IPC directly: `#[specta::specta]` on each command + `specta::Type` on the DTOs,
collected through `tauri_specta::Builder`, exported as `tauri_bindings.ts`. OpenAPI rejected (wrong transport
model). `ts-rs` rejected (types only — no typed invoke client, no events). `taurpc` rejected: its
`#[taurpc::procedures]` trait/resolver paradigm owns the IPC handler and would force a rewrite of the
`Bus<Ctx>` CQS + transport-macro layer; tauri-specta only annotates the existing `#[tauri::command]` shims.

**2. Pin the RC toolchain exactly.**
For Tauri v2 the toolchain is pre-1.0/RC: `tauri-specta = "=2.0.0-rc.25"`, `specta = "=2.0.0-rc.x"`
(matching rc.25), `specta-typescript = "=0.0.12"` — pinned with `=`. Acceptable because every piece runs at
build time, not in the shipped app: `specta-typescript` is a code printer whose output is committed and
reviewed; `specta`/`tauri-specta` drive derive + handler registration at compile/test time. The drift guard
pins generation to the exact versions, so an RC bump is a conscious, reviewed step and the committed
artifacts keep working regardless.

**3. New `@tillerd/client-bindings` package; `@tillerd/sdk` untouched; delete `sdk/orchestrator`.**
tauri-specta's output imports `@tauri-apps/api` and hardcodes the Tauri transport — incompatible
with the SDK's zero-dep, injectable-transport contract. So the generated client lives in a dedicated
`@tillerd/client-bindings` package that owns the `@tauri-apps/api` dep. The export step post-splits the
single tauri-specta output into `types.ts` (pure, import-free type declarations) and `tauri_bindings.ts`
(commands + events, `import type` from `./types`, carrying the Tauri dep), so wire shapes are consumable
without the runtime client. Since `sdk/orchestrator` is
apps/ui-only, it is deleted and its ~42 import sites re-point to `@tillerd/client-bindings`. The SDK core
(root/protocol/types) stays zero-dep and untouched. The injectable-transport abstraction is not retained for
the orchestrator surface because it had a single consumer; Tauri's own mock runtime (already used by
`command_contract.rs`) covers backend testing, and UI tests mock at the generated-client boundary.

**4. Generate the hook surface with a build-time emitter; zero per-entity config.**
A small emitter reads `tauri_bindings.ts`, discovers entities/verbs/events from the `<verb><Entity>` naming, and
emits a committed `hooks.ts`. The optimistic strategy comes from a single write-once `VERB_STRATEGY`
convention (rename→edit, archive/delete→remove, reorder→reorder, create→invalidate, …) applied across all
entities; the optimistic patch field is inferred from each command's arg type (the non-id field). The only
hand-written per-command input is a small `overrides.ts` for non-conforming commands (`session_create`
launch tail, scope-sensitive settings, `profile_create` read-back). Both `tauri_bindings.ts` and `hooks.ts` are
committed and drift-guarded. Codegen is justified here (contrary to the usual "factory beats generator" at
this scale) because the source of truth is already machine-readable and the goal is zero config.

Build-time emitter over a runtime type-level engine: a runtime engine would need heavy
template-literal/mapped types over ~120 commands (slow typecheck, hard to debug). The emitter produces
concrete typed functions, fits the existing commit-generated-file + drift-guard pattern, and keeps the
reused infra (`optimistic`, `crossWindowSync`, `client`) in place.

**5. Derive specta on the orchestrator `*View` in place; do not mirror DTOs in the transport.**
`transport_query!` returns the `*View` as the wire type today (the `|out| map` is near-identity), and the
View already exists to be the wire DTO (`#[derive(Serialize)] + serde(camelCase)`, doc: "Serializes to the
SDK wire shape"). Adding a feature-gated `specta::Type` to the same type teaches it to emit its TS shape —
one line per View, no mapping. The alternative — transport-local mirror DTOs to keep orchestrator
specta-free — was rejected: it reintroduces ~15 hand-written mirror structs + View→mirror map fns that
drift from the View, the exact duplication this change deletes. The cost of deriving in place is a
derive-only, default-off `specta` feature on the core crate; non-desktop consumers compile unchanged.

**6. Separate `specta_builder()` for export; the real handler keeps `generate_handler!`.**
Discovered at APPLY: 5 commands take a raw `serde_json::Value` in their fn signature
(`setting_get/set`, `pref_get/set`, `log_forward`); specta cannot type a `Value` parameter (only a
struct *field* can be overridden to `Unknown`), so these commands cannot enter `collect_commands!`. They
must, however, stay in the runtime handler. Resolution: `collect_transport!` is unchanged — it still
expands to `generate_handler!` and registers the full ~120-command surface for `invoke_handler` (wire
intact, `command_contract` green). A separate `specta_builder()` runs `collect_commands!` over the
specta-compatible subset purely for type export. Cost: two command lists — guarded by a test asserting
the bindings set is a subset of the handler set (folded into the drift guard). The 5 excluded commands
are host/settings ops the entity-hook emitter does not need (settings are already override-bound).

**7. Readiness preservation + scoped `sdk/orchestrator` retention (discovered at APPLY).**
The contract read of the current data layer surfaced two constraints the plan missed:
- *Readiness.* Queries/mutations gate on `whenClientReady()` (a pending promise that drives Suspense and a
  web fallback) — they do NOT call Tauri directly. Generated hooks calling tauri-specta `commands.*` raw
  would fire IPC before the orchestrator is ready and break the web-host fallback. Resolution: the emitter
  wraps every call in `whenClientReady().then(ready => ready ? commands.x(args) : fallback)`. The readiness
  singleton (`whenClientReady`/`setClient`/`getClient`) and the pure `optimistic.ts` move into
  `@tillerd/client-bindings` so generated `hooks.ts` is self-contained (correct dep direction);
  `useDesktopHost` imports `setClient` from the package. `crossWindowSync` stays in apps/ui (global
  MutationCache + Tauri-event handler, untouched — generated hooks need only correct `meta.invalidates`).
- *Scope.* `sdk/orchestrator` is apps/ui-only but is NOT all CRUD: it also holds `status`, `service-health`,
  `notifications`, `terminal-surface`, `createOrchestratorClient`, and event constants used by 37
  health/notification/terminal files. Wholesale deletion would creep into three unrelated subsystems.
  Narrowed: migrate only the CRUD entity surface (sessions/projects/workspaces hooks + the entity type
  imports `Session`/`Project`/`Workspace`/`*Args` → `@tillerd/client-bindings/types`); the non-CRUD glue
  stays in `sdk/orchestrator`. The hand-written CRUD client methods/types there are removed once unused.

**8. Hooks cover the ENTIRE command surface, classified by shape (not just 3 entities).**
The bindings (types + `commands`) already cover all ~120 commands. The hook emitter generates a hook for
EVERY command, choosing the shape by command class rather than skipping non-CRUD ones:
- query (`*List`/`*Get`/`*Resolve`/`*Search`/`*Count`) -> readiness-gated `queryOptions` + hook
- mutation (`*Create/Rename/Archive/Delete/Reorder/Move/Pin/Unpin/Duplicate/Restore/Set/Rebind/Activate/Export/Import/Discard/Seed`) -> `useMutation` with convention optimistic/invalidation
- subscription (emitted events, notification feed) -> typed `listen` hook
- stream (`surface_*`, `ipc::Channel`) -> typed channel-subscription hook (NOT Query — a byte stream is not cacheable server state)
- action (host ops: `window_*`/`file_*`/`daemon_*`/`log_*`/`pref`/`registry`) -> thin typed action wrapper
Entities covered: session, project, workspace (first), then command-library, profile, template,
launch-template, theme, keybinding, plus surface/notification/host via their non-Query shapes. Verbs the
convention doesn't map and read-back-via-list cases (e.g. profile) go through `overrides.ts`. Hooks for
features not yet UI-wired are generated-but-unconsumed (tree-shakeable), ready when those features land.

**9. One record per entity; kinds nested as option-factories (queries, mutations, channel, events).**
The emitter produces ONE export per entity/domain (`session`, `project`, `workspace`, `command`, `profile`,
`theme`, `keybinding`, `template`, `launchTemplate`, `notification`, `surface`, `setting`, `registry`,
`window`, `daemon`, `log`, `config`), each carrying nested sub-records:
```
export const session = {
  queries:   { list: (projectId?) => queryOptions(...), infinite: (...) => infiniteQueryOptions(...), get: (id) => queryOptions(...) },
  mutations: { create: () => mutationOptions(...), rename, archive, delete, reorder, ... },
};
export const surface = { queries, mutations, channel: { stream }, events: { status, exit, error } };
```
Consumed by the matching TanStack hook at the call site (never a generated `useXxxList` wrapper):
`useSuspenseQuery(session.queries.list(id))`, `ensureQueryData(session.queries.list(id))` in loaders,
`useMutation(session.mutations.rename())`, `useEventSub(surface.events.status())`.
- queries -> `queryOptions`/`infiniteQueryOptions` (queryFn in the factory so loaders + components share one unit)
- mutations -> `mutationOptions` (TanStack v5.25+, twin of `queryOptions`; plain option objects, no hooks;
  optimistic `onMutate`/`onError` + `meta.invalidates` spread in)
- channel -> typed `ipc::Channel` stream hook (surface bytes)
- events -> typed subscription factories over the generated `events` (needs Decision 10 Rust work)
Rationale: grouping by entity (not `{entity}Queries`/`{entity}Mutations` split records) gives one namespace
per domain. `queryOptions`/`mutationOptions` are the TanStack-recommended primitives; a per-operation
`useXxx` query hook traps the queryFn and can't feed loaders/prefetch (anti-pattern).

**10. Events restructured onto tauri-specta `Event` derive.**
Today events flow via hand-rolled `transport.listen("notification://event")` string channels (notification,
surface status/exit, orchestrator status). To generate a typed `events` record, the emitted events are
restructured onto tauri-specta's `#[derive(Event)]` + `collect_events!` + `builder.mount_events`, so the
bindings export a typed `events` object. This changes the live event mechanism (wire event names + typed
payloads + subscription path) and the UI subscribers (notification store, health, terminal-surface) migrate
onto the generated event record. High blast radius -- gated by the full UI test + e2e suite.

**11. Four hook kinds, classified by shape; no separate "actions" class.**
Every command is exactly one of: read -> query (`queryOptions` record), write -> mutation
(`mutationOptions` record), byte stream -> channel hook, event -> subscription record. Host/imperative
commands are NOT special-cased -- they fold into their domain's nested record by the same read/write rule:
`registry.queries`{get,list} / `registry.mutations`{set,remove}, `window.mutations`{open,focus,close},
`daemon.mutations`{ensure,disconnect,send}, `log.queries`{listLogFiles,fileSize}, `config.mutations`{reload}.
A degenerate host mutation (e.g. `window.mutations.open()` via `useMutation`) is accepted for uniformity --
one mental model (read=query, write=mutation) beats a fourth concept. The channel (surface byte stream)
and events are the only non-query/mutation shapes.

**12. Runtime TanStack client over generated records (supersedes 4, 8, 9, 11; records half of 10).**
The per-entity option-factory *records* (`session.queries.list`, `session.mutations.rename`, ...) and their
build-time emitter (`emit-factory-options.ts` -> `factory-options.gen.ts`) are dropped in favour of a small
hand-written runtime, `packages/client-bindings/src/client.query.ts`:

- `query(key, args?)` / `query.infinite(key, args?, n)` -> `queryOptions`/`infiniteQueryOptions`
- `command(key, { optimistic? })` -> `mutationOptions`; `meta.invalidates` drives the existing
  `MutationCache` invalidation in `apps/ui/app/lib/queryClient.ts`
- `reorder(key)` -> bulk reorder over the single-row `<entity>Reorder(id, sortOrder)` primitive
- `subscribe(key)` -> typed Tauri event for `useEventSub`; plus `dropById`/`reorderByIds`/`mergeById`

Args are **object-shaped** and absorbed straight from `typeof commands` (`Args<K> = Parameters<C[K]>[0]`),
results unwrapped from the `typedError` envelope via a **distributive** `OkData` conditional (a naked-union
conditional collapses to `never`). Object-param bindings come from the custom tauri-specta `LanguageExt`
exporter `apps/desktop/src-tauri/src/specta_export.rs` (kept: named args block transposed same-typed
positional args; the wire stays flat). Cache keys + default invalidation derive from the command name via one
hand `ENTITY` prefix table (plural, irregular -- not `name + "s"`) plus a `CROSS` cascade map; a coverage test
fails the build on any unclassified command (replaces the records' visible-diff safety).

Optimistic updates are **explicit opt-in** (caller passes an updater), not inferred from argument shape.
Rationale: a ~150-line runtime client + one tested name parser replaces a code-generator emitting ~360 lines
of near-identical records; less generated surface to review, the same ~zero per-call-site boilerplate, and no
runtime arg-shape magic. Events/wire decisions (8/10 event halves) are unchanged.

## Risks / Trade-offs

- **RC toolchain churn (`tauri-specta` rc.25, `specta` rc.x, `specta-typescript` 0.0.12).** Single
  maintainer; rc.24→rc.25 moved features. → Pin all three with `=`; deps are build-time only and the
  committed artifacts are what ships, so a broken bump surfaces as a failed build/drift test on a deliberate
  upgrade, never at runtime.
- **tauri-specta + the custom command macro may not compose** (attribute ordering; generated signatures with
  the trailing `bus: State<…>`; `ipc::Channel` args). → Spike the three macros against one command each first
  and confirm specta skips `State` and accepts/handles `Channel`; `command_contract.rs` proves the wire is
  unchanged. Go/no-go gate before annotating the full surface.
- **Convention may not cover every command.** Verbs outside the table or bespoke optimistic logic. → The
  `overrides.ts` map is the escape hatch; the emitter SHALL fail loudly on an unrecognized verb rather than
  silently skipping, so coverage gaps are visible.
- **Three generated files to keep in sync (`types.ts`, `tauri_bindings.ts`, `hooks.ts`).** → One drift test
  regenerates all three; the order is deterministic (`types.ts` ← split from the tauri-specta export →
  `tauri_bindings.ts` → `hooks.ts`).
- **The post-export split is a custom string transform over tauri-specta output.** A tauri-specta format
  change could break the splitter. → Keep the split minimal (extract `export type`/`export interface`
  declarations; rewrite the client to `import type`); the drift test catches a broken split immediately.
- **Deleting the injectable-transport abstraction.** Loses the TS `fakeTransport` seam for the orchestrator
  surface. → Acceptable: single consumer; UI tests mock at the generated-client boundary, backend tests use
  the Tauri mock runtime.
- **Core crate gains a feature-gated specta dependency.** → Behind `feature = "specta"`, off by default,
  enabled only by the desktop build.

## Migration Plan

1. Add deps (exact pins); spike the three macros on one command each; confirm `command_contract.rs` passes
   and `State`/`Channel` args are handled. Go/no-go.
2. Annotate all macros + hand-written shims; switch `collect_transport!` to `collect_commands!`; wire the
   `Builder` in `run()` and the contract test; add `specta::Type` derives (feature-gated) to `*View` + arg
   structs; export `tauri_bindings.ts`.
3. Create `@tillerd/client-bindings` (`@tauri-apps/api` dep); export, then split into `types.ts` +
   `tauri_bindings.ts`; add the bindings drift-guard test; commit both.
4. Build the hook emitter + `convention.ts` (`VERB_STRATEGY`) + `overrides.ts`; emit `hooks.ts`; add its
   drift-guard test; commit `hooks.ts`.
5. Migrate the ~42 `apps/ui` import sites to `@tillerd/client-bindings` + generated hooks; delete
   `packages/sdk/src/orchestrator` and the hand-written `apps/ui/app/lib/data` hooks + query-key factories
   (keep `optimistic`, `crossWindowSync`, `client`).
6. Verify end-to-end.

Rollback: additive through step 4 (generated files exist, unused). If generation proves unworkable, stop
after step 3/4 or revert; the hand-written client still functions until step 5 deletes it.

## Open Questions

- Does specta type every `*View` cleanly, or do any (`sqlx::FromRow` rows, nested enums) need a serde/specta
  attribute reconciliation?
- Final coverage line for `ipc::Channel` surface args under tauri-specta — typed or left as-is?
- Does any command's arg type defeat patch-field inference (more than one non-id field on an edit)? Such
  cases route to `overrides.ts`.

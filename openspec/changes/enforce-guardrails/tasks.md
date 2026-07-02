# Tasks — enforce-guardrails

## 1. Orphaned command surface

- [x] 1.1 Delete `diag.rs`, `store.rs`, `supervisor.rs`; unwire `lib.rs` (mod decls, managed state,
  specta entries, exit hook), the `collect_transport!` macro entries, and the contract-test cases
  (`log_forward`, `pref_get`, `pref_set`, `registry_*`, `daemon_ensure`). Regenerate bindings via
  the desktop test; rg-prove zero references.

## 2. Contract coverage

- [x] 2.1 Add the six missing contract cases (`log_list`, `log_tail`, `logs_changed_channel`,
  `logs_changed_channel_close`, `notification_channel`, `notification_channel_close`) following the
  `surface_channel` case pattern; test red-then-green if any case exposes a real arg drift.

## 3. Typed IPC ban

- [x] 3.1 Migrate `windows.ts` to the generated `windowOpen`/`windowFocus`/`windowClose` bindings;
  delete its local `invoke` helper.
- [x] 3.2 Add ast-grep rule `no-raw-invoke` (error) scoped to `apps/ui/app/**` +
  `packages/client-bindings/src/**`, excluding `tauri_bindings.gen.ts` and
  `lib/transport/core.ts`; add valid/invalid rule tests.

## 4. Layer rule flip

- [x] 4.1 Remove the `crate::infra::daemon_pty_api` import from `shared/bus.rs`'s test module
  (app-owned edge or local fake, preserving the test's observable assertions); `ast-grep scan`
  shows zero `infra-only-in-app` findings.
- [x] 4.2 Flip `infra-only-in-app` severity `warning` -> `error`; rule tests updated.

## 5. Consistency

- [x] 5.1 Dedupe the `0x00` domain-channel byte tag: `spawn_logs_watcher` uses the sink-owned
  tag helper; the layout is declared once.
- [x] 5.2 `profile_create`: adopted `transport_create!` via a new `GetProfile` by-id query
  (unit-tested); wire shape unchanged.

## 6. Fix-all gate

- [x] 6.1 rg sweep for removed command names; `bun run verify`, `ast-grep scan` (0 errors,
  0 `infra-only-in-app` findings), `ast-grep test`, full e2e suite green.

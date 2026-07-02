## 1. Spike: prove tauri-specta composes with the transport macros (go/no-go)

- [x] 1.1 Added RC toolchain pinned exact in root `[workspace.dependencies]` (`specta`/`specta-typescript`/`tauri-specta` all `=2.0.0-rc.25`/`=0.0.12`) + referenced in desktop crate; resolves against tauri 2.11.3, builds
- [x] 1.2 Annotated `transport_command!` macro + `surface_create` + `ProjectView` (`specta::Type`); compiles
- [x] 1.3 Linchpin verified: `#[specta::specta]` composes inside the macro, skips `bus: State`, and accepts `ipc::Channel<Vec<u8>>` + generic `<R>` + `AppHandle` (surface_create compiled). GO
- [x] 1.4 Annotation-level export path proven by the compiles above; full `Builder` + `collect_commands!` export validated end-to-end in task 3 (where the whole surface is registered)

## 2. Rust: annotate the full IPC surface

- [x] 2.1 Added `#[specta::specta]` to all 3 transport macros
- [x] 2.2 Added `#[specta::specta]` to the hand-written shims (surface_*, window/files/diag/bridge/menu/supervisor/orchestrator_host/notification/settings)
- [x] 2.3 Feature-gated `specta::Type` on all 12 `*View` DTOs + nested entity types (LaunchSpec, Profile, Theme, KeybindingEntry, …); `specta` feature wired on orchestrator, enabled from desktop
- [x] 2.4 Reconciled: `serde_json::Value` recurses infinitely under specta → `SettingView.value` overridden to `Unknown` (`specta(type = ...)`); the 5 commands with raw `Value` in their fn signature (`setting_get/set`, `pref_get/set`, `log_forward`) can't be field-overridden, so excluded from the bindings (still registered in the handler — see 2.5)
- [x] 2.5 DIVERGENCE from plan: did NOT flip `collect_transport!` (it stays `generate_handler!` for the real `invoke_handler`, registering the full ~120-command surface unchanged). Instead added a separate `specta_builder()` (`collect_commands!` over the specta-compatible subset) for export only. Reason: the 5 `Value`-arg commands must stay in the handler but cannot enter `collect_commands!`. Trade-off: two command lists (handler vs bindings) → needs a guard test that bindings ⊆ handler (folded into 3.4). command_contract still drives the full handler; green
- [ ] 2.6 Events (`collect_events!` + `#[derive(Event)]`) — DEFERRED, not yet done; `ipc::Channel` surface args accepted by specta (surface_* in bindings)

> **Sections 3-5 below describe the abandoned build-time emitter/records plan and the `types.ts` split.**
> At APPLY this pivoted to a runtime client (design Decision 12). Real completed work is in **Section 7**;
> 3-5 are kept for history. The `@tillerd/sdk/orchestrator` + `apps/ui/app/lib/data` deletions (5.2/5.3)
> already landed in earlier commits.

## 3. New package + bindings export + drift guard

- [ ] 3.1 Create `packages/client-bindings` (`@tillerd/client-bindings`) with `@tauri-apps/api` as a dependency and the package wiring (exports, tsconfig, turbo)
- [ ] 3.2 Build the `tauri_specta::Builder` in `lib.rs run()`; register `builder.invoke_handler()`; `mount_events`; migrate the command-contract test to the same builder
- [ ] 3.3 Export the tauri-specta output into `packages/client-bindings` under `#[cfg(debug_assertions)]` and from a test; add the post-export split that extracts the `export type`/`export interface` declarations into a pure import-free `types.ts` and rewrites the client to `tauri_bindings.ts` (`import type` from `./types`)
- [ ] 3.4 Add a bindings drift-guard test (regenerate + split, fail on diff for `types.ts` and `tauri_bindings.ts`); run the desktop suite; confirm `command_contract.rs` + drift guard pass; commit both files

## 4. Hook emitter (zero per-entity config)

- [ ] 4.1 Write `convention.ts` — the write-once `VERB_STRATEGY` table (rename→edit, archive/delete→remove, reorder→reorder, create→invalidate, pin/unpin→edit, move/duplicate/restore→invalidate)
- [ ] 4.2 Write `overrides.ts` — entries for the known non-conforming commands (`session_create` launch tail, scope-sensitive settings, `profile_create` read-back)
- [ ] 4.3 Build the emitter: read `tauri_bindings.ts` (commands/events) + `types.ts` (arg/return types), group commands/events by `<verb><Entity>`, apply `VERB_STRATEGY`, infer the optimistic patch field from each arg type (non-id field), apply overrides; emit `hooks.ts` with typed query/mutation/event hooks reusing the existing `optimistic`/`crossWindowSync`/`client` infra and `meta.invalidates`. Fail loudly on an unrecognized verb (no silent skip)
- [ ] 4.4 Add unit tests for the emitter: convention mapping per verb; patch-field inference; override application; unrecognized verb fails; emitted hooks typecheck against the bindings
- [ ] 4.5 Add a hooks drift-guard test (regenerate, fail on diff); commit `hooks.gen.ts`
- [ ] 4.6 Extend the emitter to the FULL command surface, classified by shape (query/mutation/subscription/stream/action) so nothing is skipped: add entities command-library/profile/template/launch-template/theme/keybinding (query+mutation hooks); surface (channel-subscription hook); notification (list query + mutations + subscription); host ops (thin action wrappers). Add `VERB_STRATEGY` entries for pin/unpin/duplicate/restore/search/activate/export/import/rebind/seed; route read-back-via-list (profile) + non-conforming verbs through `overrides.ts`. tsc + emitter tests stay green

## 5. Migrate apps/ui; delete the hand-written layers

- [ ] 5.1 Re-point the ~42 `apps/ui` import sites from `@tillerd/sdk/orchestrator` to `@tillerd/client-bindings` and the generated hooks
- [ ] 5.2 Delete `packages/sdk/src/orchestrator` (apps/ui-only); confirm `@tillerd/sdk` core (root/protocol/types) is untouched and still zero-dep (`packages/logger` + sdk tests still build)
- [ ] 5.3 Delete the hand-written hooks + query-key factories in `apps/ui/app/lib/data` (keep `optimistic`, `crossWindowSync`, `client`); fix resulting type errors
- [ ] 5.4 Run the UI test suite + typecheck; confirm sidebar query loads and mutation flows (create/rename/archive/delete/reorder) behave unchanged

## 6. Verify end-to-end

- [ ] 6.1 Run the app; exercise create/rename/archive/delete/reorder for a project and a session; confirm optimistic UI + cross-window invalidation still work
- [ ] 6.2 Confirm a deliberate Rust `*View` field change forces regeneration: the bindings + hooks drift tests fail until regenerated, and the change surfaces as a TS type error at the hook call site
- [ ] 6.3 Confirm adding a new conforming Rust command produces a working hook after regeneration with no hand edit beyond the Rust definition

## 7. APPLY pivot: runtime client (actual state)

- [x] 7.1 Object-param bindings via custom `LanguageExt` exporter `specta_export.rs` (wired into `lib.rs` + `command_contract.rs`); flat wire preserved
- [x] 7.2 Fix exporter multiline-payload bug (invoke arg `{ .. }` on a trailing line left a stale destructure); rewrite payloads in a whole-file pass; regenerate `tauri_bindings.gen.ts`
- [x] 7.3 Write `client.query.ts`: `query`/`query.infinite`/`command`/`reorder`/`subscribe` + `dropById`/`reorderByIds`/`mergeById`; args from `typeof commands`; results via distributive `OkData`; readiness-gated; `meta.invalidates`
- [x] 7.4 Coverage test (every `keyof commands` classifies via `ENTITY`) + key/invalidation tests -- 12 pass
- [x] 7.5 Migrate `apps/ui` consumers (records -> `query`/`command`/`reorder`; positional `commands.*` -> object args)
- [x] 7.6 Delete `factory-options.gen.ts`, `emit-factory-options.ts`, `runtime-spike.ts`, dead `optimistic.ts`; prune `package.json` exports
- [x] 7.7 Gates green: `apps/ui` tsc 0 errors; client-bindings 12 + UI 202 tests pass; `cargo test command_contract` 3/3 (wire+drift); `sg scan` clean
- [x] 7.8 `bun run verify` green (format/check-types/lint/test) + `cargo test --workspace` all pass (incl. desktop 56; fixed `notification_response_matches_sdk_notification_shape` for the single-type wire shape)
- [ ] 7.9 e2e (`turbo run e2e`) -- heavy, not yet run

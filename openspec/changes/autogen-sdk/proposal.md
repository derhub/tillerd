## Why

The desktop IPC surface is ~120 commands across ~12 entities, and every layer above it is hand-maintained: the orchestrator wire types in `packages/sdk/src/orchestrator`, the `invoke(method, args)` mapping, and one TanStack hook per operation (`useCreateSession`, `useRenameProject`, …). Three copies of the same contract drift independently — a Rust `*View` field rename is caught only at runtime. The contract already lives in Rust (the `app/` CQS types + the transport macros); it should be generated downward to a single committed, drift-guarded artifact, not retyped three times. Adding a command in Rust should make its typed hook appear with no hand edit.

## What Changes

- Adopt **tauri-specta** (pinned exact, RC toolchain) to auto-generate TypeScript bindings — commands, queries, and events — from the Rust desktop IPC layer.
  - Annotate the `transport_command!` / `transport_query!` / `transport_create!` macros with `#[specta::specta]` so all ~90 macro-generated domain shims are covered by three edits; annotate the ~30 hand-written host/settings/surface/notification shims; switch `collect_transport!` to expand to `tauri_specta::collect_commands![...]`.
  - Add feature-gated `specta::Type` derives to the `*View` wire DTOs and the `Deserialize` arg structs; export emitted events via `collect_events!` + `#[derive(Event)]`.
  - Export through `tauri_specta::Builder` in `lib.rs run()`.
- **New package `@tillerd/client-bindings`** owns the generated client. The export step splits the output into `types.ts` (pure wire types, **zero imports** — no `@tauri-apps`) and `tauri_bindings.ts` (commands + events, `import type` from `./types`), the latter carrying the `@tauri-apps/api` runtime dep. `@tillerd/sdk` stays zero-dep and transport-agnostic — **untouched**.
- **Generate the TanStack hook surface** with a build-time emitter: a script reads `tauri_bindings.ts` + `types.ts` + a write-once `VERB_STRATEGY` convention table + a small `overrides.ts`, and emits a committed, drift-guarded `hooks.ts`. Entities/verbs/events are **discovered** from the bindings (`<verb><Entity>` grouping); the optimistic patch field is **inferred** from each command's arg type (the non-id field). **Per-entity config is zero** — only non-conforming commands (`session_create` launch tail, scope-sensitive settings, `profile_create` read-back) get an override entry.
- **BREAKING (internal):** delete `packages/sdk/src/orchestrator` (consumed only by `apps/ui`); migrate the ~42 `apps/ui` import sites to `@tillerd/client-bindings` and the generated hooks. Wire shape stays byte-identical — `command_contract.rs` is the contract.
- Drift guards: a test regenerates `types.ts`, `tauri_bindings.ts`, **and** `hooks.ts` and fails if any differs from the committed files.

## Capabilities

### New Capabilities

- `generated-ipc-bindings`: auto-generated, drift-guarded TypeScript bindings (commands, queries, events) produced from the Rust desktop IPC layer via tauri-specta, housed in a dedicated `@tillerd/client-bindings` package; the single source of truth for the desktop wire types and the typed invoke client.
- `generated-entity-hooks`: a build-time emitter that produces the committed, drift-guarded TanStack hook surface (queries, mutations, event subscriptions) from the bindings plus a write-once verb→strategy convention and a small overrides map — zero per-entity configuration.

### Modified Capabilities

- `client-engine`: the per-entity TanStack surface (query, mutation, and event hooks) is produced by the generated hook emitter instead of being hand-written one operation at a time; existing mutation semantics (query keys, `meta.invalidates`, optimistic snapshot/apply/rollback, global settle-invalidate) are preserved.

## Impact

- Rust: `apps/desktop/src-tauri` (`transport/macros.rs`, `transport/domain.rs`, the hand-written `*_host` shims, `lib.rs`, `command_contract.rs`); `crates/orchestrator` `*View` DTOs gain a feature-gated derive.
- Deps: add `specta = "=2.0.0-rc.x"`, `specta-typescript = "=0.0.12"`, `tauri-specta = "=2.0.0-rc.25"` (exact pins) to the desktop crate; `@tauri-apps/api` to the new `@tillerd/client-bindings` package.
- TS: new `packages/client-bindings` (generated `types.ts` + `tauri_bindings.ts` + `hooks.ts`; write-once emitter + `convention.ts` + `overrides.ts`); `packages/sdk/src/orchestrator` deleted; ~42 `apps/ui` files re-pointed; `apps/ui/app/lib/data` hand-written hooks + query-key factories removed (kept: `optimistic`, `crossWindowSync`, `client`).
- `@tillerd/sdk` core (root/protocol/types — used by `packages/logger`, `apps/ui`, sdk tests) untouched and still zero-dep.
- Generated `types.ts`, `tauri_bindings.ts`, and `hooks.ts` are committed; CI fails on drift.

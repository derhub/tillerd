## Why

The orchestrator `app/` layer is the real feature surface: 113 use-case handlers across 8 domains. Only 31 are reachable from the desktop client — the other 82 have no tauri command, so a renderer (current or future) cannot call them. The `template` domain is the extreme case: 12 handlers, none exposed, and the module is not even in `app/mod.rs`, so an entire built feature is dead. A feature that exists in the core but not in the client is invisible: the API must exist in the desktop client first, so UI can be built against it later. This change closes the gap — every app handler gets its desktop command, UI-optional.

## What Changes

- Wire the unwired domain: add `pub mod template;` to `app/mod.rs` (it already compiles).
- Expose all 82 unexposed handlers as desktop tauri commands via the existing transport shims:
  - `transport_command!` for state mutations returning `()` (~55).
  - `transport_query!` for reads, mapping core output to the wire `*View` DTO (~24).
  - `transport_create!` for creates: mint id, execute, read back the entity by id (~3 — `launch_template_create`, `profile_create`, and any remaining caller-id create).
- Register each command in `collect_transport!` (`transport/macros.rs`) and add it to `command_contract.rs`: the handler list plus a `cases` entry with a representative argument JSON, so the existing `every_desktop_ipc_command_is_registered_and_accepts_its_arg_shape` contract test covers every new command.
- Add an SDK response-shape test (`assert_keys` over the camelCase key set) for each new `*View` return type, mirroring the existing project/session/command/workspace shape tests in `transport/domain.rs`.
- Fold in the residual dead-code cleanup so `clippy -D warnings` reaches green once handlers are live: redundant `entities/mod.rs` and `infra/mod.rs` flat re-exports, and entity helpers that remain genuinely unused after exposure.
- No ACL/capabilities change: the renderer invokes from the `tauri://localhost` local origin, which skips per-command ACL; `capabilities/default.json` is untouched.

## Capabilities

### New Capabilities

- `desktop-command-coverage`: every orchestrator app-layer command and query is reachable from the desktop client as a registered tauri command with a stable wire argument shape and (for queries/creates) a `*View` response shape asserted against the SDK contract; the registration-and-arg-shape contract test enumerates all of them.

### Modified Capabilities

<!-- None. This adds client reach to existing core behavior; no command's domain
     semantics, wire field names, or response shapes change. -->

## Impact

- `crates/orchestrator/src/app/mod.rs` — `pub mod template;` (wire the launch-template domain).
- `apps/desktop/src-tauri/src/transport/domain.rs` — 82 new `transport_command!`/`transport_query!`/`transport_create!` shims + their `*View` shape tests.
- `apps/desktop/src-tauri/src/transport/macros.rs` — `collect_transport!` gains the 82 command idents.
- `apps/desktop/src-tauri/src/command_contract.rs` — handler list + `cases` entries for the 82 new commands.
- `crates/orchestrator/src/{entities,infra}/mod.rs` and unused entity helpers — residual dead-code removal to reach clippy-green.
- Wire / SDK: additive only — new commands, no change to existing command names, args, or response JSON.

## Dependencies

- **Sequenced after `client-assigned-create-ids`.** That change (a) adds the caller-`id` field to `NewLaunchTemplateCmd` so `transport_create!` can wire `launch_template_create` (today it mints internally and is incompatible with the create macro), and (b) replaces `command_contract.rs`'s hand-copied `generate_handler!` list with `collect_transport!()`. Adding 82 entries before it lands would collide on both surfaces. APPLY of this change MUST wait until `client-assigned-create-ids` merges.

## Context

The orchestrator `app/` layer holds 113 command/query handlers across 8 domains; the desktop client exposes only 31 as tauri commands. The other 82 are unreachable from any renderer, and the entire `template` domain (12 handlers) is not even in `app/mod.rs`. The transport machinery to expose them already exists — `transport_command!`/`transport_query!`/`transport_create!` shims in `transport/domain.rs`, listed in `collect_transport!`, contract-tested in `command_contract.rs`. This change applies that existing machinery to the remaining 82 handlers. No new transport mechanism is invented.

In-force ADRs: 0033 (two-plane storage), 0036/0038 (de-abstracted storage, infra raw + app owns domain), 0037 (event dispatch). This change adds client reach over the existing app layer; it changes no domain rule and no wire contract.

## Goals / Non-Goals

**Goals:**

- Every app command/query is a registered desktop command with a stable wire arg shape and (for queries/creates) an SDK-asserted `*View` response.
- The registration-and-arg-shape contract test enumerates all of them.
- `clippy -D warnings` reaches green once the previously-dead handlers go live.

**Non-Goals:**

- No UI. This delivers the client API only; building screens against it is later work.
- No change to existing command names, args, response JSON, or domain semantics.
- No ACL/capabilities change (local origin skips per-command ACL).
- No new transport macro or dispatch mechanism — the existing shims are reused verbatim.

## Decisions

### D1 — Reuse the three existing shim macros

Each handler maps to exactly one shim by its CQS kind: a mutation returning `()` -> `transport_command!`; a read -> `transport_query!` mapping core output to the `*View`; a create -> `transport_create!` (mint id, execute, read back by id). The 82-way split: ~55 command, ~24 query, ~3 create. No handler needs a hand-written shim except where a create is not yet caller-id (see D4).

### D2 — Wire the template domain first

`app/mod.rs` gains `pub mod template;`. The 12 launch-template/template-library handlers then compile and are exposed like any other domain. This single line is what turns the dead `Template`/`LaunchTemplate`/`LaunchTemplateRepo` code live.

### D3 — Contract coverage is the gate

Every new command is added to `collect_transport!` AND to `command_contract.rs` (handler list + a `cases` entry with a representative arg JSON). The existing `every_desktop_ipc_command_is_registered_and_accepts_its_arg_shape` test then proves registration + arg-shape for all 82. Each new `*View` return type gets an `assert_keys` shape test mirroring the existing project/session/command/workspace tests, so the SDK contract is enforced.

### D4 — Sequenced after `client-assigned-create-ids`

`client-assigned-create-ids` must land first. It (a) adds the caller-`id` field to `NewLaunchTemplateCmd` (today it mints internally, so `transport_create!` — which mints at the transport and passes the id in — cannot wire `launch_template_create`), and (b) replaces `command_contract.rs`'s hand-copied `generate_handler!` list with `collect_transport!()` (so the contract handler list stops drifting). Adding 82 entries before it lands would collide on both. *Alternative rejected:* hand-write a list-diff create shim for `launch_template_create` now and rebase the contract list later — that duplicates work the prerequisite already does and invites a messy merge. So APPLY waits for the prerequisite.

### D5 — Fold in the residual clippy cleanup

Wiring the handlers live clears most dead-code, but redundant `entities/mod.rs` and `infra/mod.rs` flat re-exports (internal code uses full paths) and a few entity helpers stay unused. Remove them in this change so `clippy -D warnings` — the lint gate — passes green. This is the same residual debt that was left untouched when infra-raw landed (the branch was clippy-red on it); exposing the features is the natural point to clear it.

## Risks / Trade-offs

- **82 commands is a large diff** -> the work is mechanical and uniform (one shim + one contract case + one shape test per handler); the contract test is the single safety net proving all of them register and accept their args.
- **Applying before the prerequisite** -> blocked by D4; the apply gate is the prerequisite's merge, not just the user's go.
- **SDK shape drift** -> every `*View` gets an `assert_keys` test, so a key-set change fails the build.
- **A handler's arg shape is wrong in `cases`** -> the contract test catches an `invalid args` response immediately.

## Migration Plan

After `client-assigned-create-ids` lands: wire `app/template`, then per domain add the shims + `collect_transport!` idents + `command_contract.rs` cases + `*View` shape tests, run the contract test green per domain, then the residual clippy cleanup, then `bun run verify` + `bun run e2e` + `ast-grep`. No data or wire-format migration; additive only.

## Open Questions

- None blocking. The per-command argument shapes are read directly off each handler's fields; the `*View` shapes off the existing view types.

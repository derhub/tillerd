## Why

The Rust orchestrator owns the backend (ADR-0022) but the agent adapter parse functions (hook → status/content mapping) still live in TypeScript, the gate fan-out is not yet wired into the orchestrator's surface runtime, the agent surface UI does not exist (only `DesktopTerminalPane`), and hook setup is not invoked at spawn or cleaned up at remove. These four gaps prevent any real agent-driven workflow from completing end-to-end.

## Already Built

The following infrastructure exists and is not part of this change's scope:

- **Gate hook ingress + fan-out** — `apps/gate` receives hook payloads, normalises them via `V1Adapter`, and publishes per-session broadcast events through `Subscriptions`.
- **Gate subscribe face** — `apps/gate/src/endpoint/subscribe.rs` streams `HookEvent`s to subscribers; the wire protocol is negotiated with `HOOK_SUBSCRIPTION_WIRE_VERSION`.
- **`gate-client` subscription API** — `crates/gate-client` provides `encode_subscribe_preamble`, `decode_subscription_frame`, and `negotiate_ready`; already a dependency of `crates/orchestrator`.
- **`HookEvent` + `HookKind`** — all six variants defined in `crates/contracts/src/lib.rs`.
- **`SurfaceKind::Agent`** — the `Agent` variant already exists in the persistence enum; no migration needed.
- **TS hook-parse reference implementations** — `packages/sdk/src/hook-content.ts` (`hookEventToContent`) and `packages/engine/src/session/status.ts` (`StatusMapper` with the six-variant status map) are the authoritative reference for the Rust port; they remain in place as dead code after this change.
- **TS hook setup** — `packages/adapter-claude-code/src/setup.ts` implements idempotent install/uninstall with marker `"tillerd-notify"`; it is the reference for the Rust port in `agent/setup.rs`.
- **Terminal surface runtime** — `SurfaceRuntime`, `open_terminal`, and the existing proxy-task pattern in `crates/orchestrator/src/surface/runtime.rs` are the template for `open_agent`.

## What Changes

- **Rust agent module** — new `crates/orchestrator/src/agent/` module containing: `AgentDefinition` struct + hardcoded `AGENT_DEF` constant (ported from `packages/adapter-claude-code/src/index.ts`); `hook_to_status(HookEvent) -> AgentStatus` and `hook_to_content(HookEvent) -> Option<ContentEvent>` (ported from the TS references above); `install`/`uninstall` setup functions (ported from `packages/adapter-claude-code/src/setup.ts`).
- **Hook-event routing path** — the orchestrator surface runtime opens a gate subscription (per `surface_id`) when an agent surface is opened, receives fan-out hook events, calls the Rust parse functions, and emits typed status + content events through `SurfaceEventSink`. The subscription task is cancelled via `JoinHandle::abort()` (matching the existing terminal-proxy teardown pattern) on surface remove.
- **Agent surface UI** — a new `AgentPane` component rendered for `surface.kind = agent`; shows an embedded terminal pane (raw PTY bytes) plus a status badge (`IDLE | WORKING | WAITING_INPUT | DONE | crashed`) derived from hook events, a content stream (tool-use entries), and typed failure states (process exit with non-zero qualifier, gate subscription error).
- **Idempotent hook setup wired to spawn/remove** — the `open_agent` path in the orchestrator calls the Rust `setup::install` before spawning the PTY; `remove` calls `setup::uninstall`. Both are idempotent and coexist with any user-owned hooks already in the agent settings file.

## Capabilities

### New Capabilities

- `agent-adapter-rs`: Rust `AgentDefinition` struct and hook-event parse functions (hook → `AgentStatus`; hook → `Option<ContentEvent>`); provides the Rust equivalents of the TS references, which become dead code.
- `hook-event-routing`: Orchestrator subscribes to gate fan-out per `surface_id` on agent-surface open; routes decoded `HookEvent`s through parse to `SurfaceEventSink` status + content callbacks; subscription torn down on remove.
- `ui-agent-pane`: Agent surface UI component — terminal pane embedded, status badge, content stream, failure states.
- `agent-hook-lifecycle`: Hook setup invoked at `open_agent` (before spawn), teardown at `remove`; idempotent, coexists with user hooks.

### Modified Capabilities

- `surface-runtime`: `open_agent` method added (alongside `open_terminal`); agent surfaces additionally open a gate subscription and trigger hook setup. Remove path extended with gate-task abort and hook teardown for agent surfaces.

## Impact

- `crates/contracts` — add `AgentStatus` enum and `ContentEvent` struct; no protocol changes.
- `crates/orchestrator` — new `agent` module (`definition.rs`, `parse.rs`, `setup.rs`, `mod.rs`); `surface/runtime.rs` gains `open_agent`, gate-drain task (using `JoinHandle::abort()` for teardown), and `on_content`/`on_error` default-no-op methods on `SurfaceEventSink`; `surface/api.rs` gains `create_agent_surface`.
- `apps/ui` — new `AgentPane` component; `lib/panelTree.ts` gains `agent` content type; `AppShell.tsx` `renderContent()` dispatch updated to route `agent` to `AgentPane`; Tauri event bindings for `surface:content` and `surface:error`.
- `packages/sdk/src/hook-content.ts`, `packages/engine/src/session/status.ts`, `packages/adapter-claude-code/src/setup.ts` — become dead code (not deleted; formal retirement deferred to TS engine removal).
- No daemon or gate protocol changes; `gate-client` subscription API is already present and wired.

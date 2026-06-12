## Context

The Rust orchestrator crate (`crates/orchestrator`) already owns the surface runtime for terminal surfaces (ADR-0022/0024). The `SurfaceRuntime` and `SurfaceApi` types exist, `open_terminal` is implemented, and the `SurfaceEventSink` trait exposes `on_bytes`, `on_status`, and `on_exit`. The `gate-client` crate can subscribe to gate fan-out per `surface_id` and decode `SubscriptionFrame` values.

Hook setup lives in `packages/adapter-claude-code/src/setup.ts`: idempotent install/uninstall that writes the notify-binary hook command into the agent's settings file using marker `"tillerd-notify"`. The TS parse references are `packages/sdk/src/hook-content.ts` (`hookEventToContent`, `PostToolUse` → `ContentEvent`) and `packages/engine/src/session/status.ts` (`StatusMapper` with the six-variant `HookKind` → `SessionStatus` map). These produce the same contract that `contracts` defines in Rust (`HookEvent`, `HookKind`).

The `contracts` crate already defines `HookEvent` and `HookKind` variants (all six events). `AgentStatus` does not yet exist in `contracts`. The `SurfaceEventSink` trait does not yet have `on_content` or `on_error`.

The UI has `DesktopTerminalPane` and `TerminalPane` but no agent surface component.

## Goals / Non-Goals

**Goals:**

- Add `AgentStatus` and `ContentEvent` to the `contracts` crate.
- Implement hook → status and hook → content parse functions in `crates/orchestrator/src/agent/` (new module, no new crate).
- Add `AgentDefinition` struct to `crates/orchestrator/src/agent/`.
- Extend `SurfaceEventSink` with `on_content` and `on_error`.
- Implement `open_agent` on `SurfaceRuntime` and `SurfaceApi`: gate register → hook install → daemon spawn, with a background task that drains the gate subscription and calls `on_status`/`on_content`/`on_error`.
- Extend `SurfaceRuntime::remove` and `SurfaceApi::remove` to cancel the gate subscription task and run hook uninstall for agent surfaces.
- Wire the hook-setup logic into Rust (call the TS setup through the Tauri host or reimplement the two-file manipulation steps in Rust — see Decisions).
- Add `AgentPane` to `apps/ui` with status badge, content stream, and failure states; update panel dispatch.
- Add Tauri event bindings for `on_content` and `on_error`.

**Non-Goals:**

- Retiring the TS `adapter-claude-code` package files (they become dead code; formal retirement is a later change).
- Implementing transcript read-on-hook content (the `agent-content` spec pattern using file reads). This change sources content events solely from `PostToolUse` hook payload — no transcript file reads.
- Multi-user or multi-subscription scenarios; one gate subscription per surface.
- Gate supervision or gate restart recovery (gate is assumed to be running; typed error on failure).

## Decisions

### 1. Hook-setup in Rust vs. calling TS at runtime

**Decision:** Reimplement the settings-file mutation (install/uninstall) in Rust inside `crates/orchestrator/src/agent/setup.rs`.

**Rationale:** The setup logic is pure filesystem manipulation (read JSON, modify a map, write atomically with a backup) with no TS-specific dependencies. Calling back into TS from a Rust `open_agent` path would require a callback trait or Tauri channel, adding coupling. Keeping it in Rust makes the orchestrator self-contained and testable without a Tauri host. The TS implementation is the reference; the Rust version uses the same marker constant (`tillerd-notify`) and hook-event list so it is byte-compatible.

**Alternative considered:** Expose a `SetupBridge` trait on `SurfaceApi` and let the host (Tauri) call the existing TS `setup.install`. Rejected: adds a round-trip through the Tauri command bus for a synchronous I/O operation, complicates testing, and contradicts ADR-0022 (orchestrator owns the backend).

### 2. Gate subscription task lifecycle

**Decision:** Spawn a `tokio::task` per agent surface inside `SurfaceRuntime::open_agent`. The task holds the gate TCP/Unix connection and loops decoding `SubscriptionFrame`s. The `JoinHandle` is stored in the proxy map entry and `.abort()` is called on `remove`, matching the existing terminal-proxy teardown pattern in `SurfaceRuntime`.

**Rationale:** The gate subscription is a long-lived read loop that runs concurrently with PTY I/O. A dedicated task per surface matches the existing pattern (`SurfaceRuntime` already spawns one proxy task per terminal surface and aborts it on remove). `JoinHandle::abort()` is already used throughout the runtime — adding `tokio-util` for `CancellationToken` would be an unnecessary new dependency for no additional benefit in this single-task case.

**Alternative considered:** A single multiplexed gate subscriber task that fans out by `surface_id`. Rejected: more complex than needed for the 0.0.3 scope; one task per surface fits the existing per-proxy model.

### 3. SurfaceEventSink extension: on_content and on_error

**Decision:** Add `on_content(&self, surface: &SurfaceId, event: &ContentEvent)` and `on_error(&self, surface: &SurfaceId, reason: &str)` to the `SurfaceEventSink` trait as required methods with default no-op implementations.

**Rationale:** Existing implementors (Tauri host, test doubles) need to opt in to the new callbacks without breaking. Default no-op bodies (returning `()`) satisfy existing `NullSink` and test doubles without a breaking change. The Tauri host overrides both to emit Tauri events to the frontend.

**Alternative considered:** A separate `AgentEventSink` trait. Rejected: doubles the trait infrastructure; the surface runtime already holds one `Arc<dyn SurfaceEventSink>`, and adding two methods is simpler than storing two sinks.

### 4. AgentDefinition binding

**Decision:** A single hardcoded `AGENT_DEF: AgentDefinition` constant in `crates/orchestrator/src/agent/definition.rs` mirrors the TS `claudeCode` export. `SurfaceApi::create_agent_surface` uses it directly; no registry or dynamic dispatch for now.

**Rationale:** There is exactly one adapter in scope (the coding-agent CLI). A registry or `dyn AgentAdapter` trait is premature until a second adapter exists. The constant is the minimal change; a registry can be layered later without breaking the `create_agent_surface` API.

**Alternative considered:** A `Box<dyn AgentAdapter>` injected into `SurfaceApi`. Rejected: adds lifetime and object-safety complexity for one concrete type; YAGNI at pre-v1.

### 5. ContentEvent type placement

**Decision:** Add `ContentEvent` and `AgentStatus` to `crates/contracts/src/lib.rs`.

**Rationale:** `contracts` is the single cross-language wire-type crate; both types are part of the observable contract between the orchestrator backend and the Tauri frontend. Placing them in `orchestrator` would make them internal and force the host to depend on `orchestrator` for types — contrary to the dependency arrow (host depends on `orchestrator`, which in turn depends on `contracts`).

### 6. UI: AgentPane layout

**Decision:** `AgentPane` renders as a vertical split: top 80% is the embedded `TerminalPane` (raw PTY bytes), bottom 20% is a scrollable content list. The status badge overlays the top-right corner of the terminal area. The failure indicator renders as a dismissible banner above the content list.

**Rationale:** The terminal output is the primary focus for a coding-agent surface; content events are supplementary. The proportional split keeps the terminal dominant while surfacing structured content without requiring a separate panel. Implemented as fixed CSS proportions (`flex-grow: 4` terminal, `flex-grow: 1` content). User-resizable divider is deferred to a design polish pass at 0.1 or later; no component-level geometry control is added in this change.

### 7. AgentPane split ratio

**Decision:** Fixed 80/20 split (CSS `flex-grow: 4` terminal, `flex-grow: 1` content list). No user-resizable divider in 0.0.3.

**Rationale:** The panel engine (`panelTree.ts`) already supports arbitrary splits; a component-level resizable divider would duplicate that concern. The fixed ratio keeps the terminal dominant, matches the existing `TerminalPane` embedding pattern, and can be adjusted without architectural changes in a follow-on pass. Roadmap-plan decision #5 explicitly defers geometry refinement per version.

### 8. Hook-setup backup convention

**Decision:** Rust `setup::install` writes exactly one backup file named `settings.json.bak`, overwritten on each install call.

**Rationale:** The TS reference implementation creates timestamped backup files (`${p}.tillerd-backup-${ts}`), producing unbounded accumulation on repeated installs. A single overwritten `.bak` file is simpler, prevents disk accumulation, and satisfies the recovery requirement (restore if install goes wrong). Roadmap-plan decision #18 requires idempotent setup; unbounded side effects contradict that. The Rust implementation is the reference going forward; the TS behaviour is not replicated.

### 9. on_error banner copy

**Decision:** The `AgentPane` error banner displays the raw `reason` string from the `surface:error` event with no category-specific copy. A `[Dismiss]` button clears the banner; the terminal pane remains visible.

**Rationale:** Roadmap-plan decision #13 establishes the failure-state pattern: a failed item gets its pane in an error state with a dismissible indicator. Category-specific copy (e.g. "gate unavailable" vs "auth rejected") is a UX polish concern deferred to post-0.0.3 review. Passing the raw reason string through keeps the implementation minimal and still surfaceable to the user.

## Risks / Trade-offs

- **Hook-setup reimplementation drift** — The Rust setup and the TS setup must stay in sync (same marker, same event list, same backup behaviour). As long as both use `HOOK_MARKER = "tillerd-notify"` from a shared constant source (or the TS files are eventually deleted), drift is bounded. Mitigation: cross-language fixture test that exercises both and asserts the same settings-file output.
- **Gate subscription on agent open is synchronous** — If the gate is slow to accept the connection, `open_agent` blocks. Mitigation: the gate is a local Unix socket; timeouts should be sub-millisecond. A short connect timeout (e.g. 2s) is applied and surfaces as a typed error.
- **No transcript reads in 0.0.3** — Content events come only from `PostToolUse` payload; large tool responses are not paginated. This is acceptable for 0.0.3; transcript read-on-hook is a follow-on.
- **AgentPane content list is unbounded in memory** — A long-running agent surface accumulates content entries indefinitely. Mitigation: cap at a fixed maximum (e.g. 500 entries, drop oldest) in the UI component; this is a UI-local bound, not persisted.

## Migration Plan

No schema migration needed. The `surfaces` table already has a `kind` column; `agent` is a new enum variant not previously written. On first use the new code path writes `kind = 'agent'` rows; prior rows are unaffected.

The TS hook-parse functions (`hookEventToContent`, status mapping) remain in their files but are no longer called from any active code path. They are dead code, not deleted, to keep the diff minimal and allow a clean audit before the TS engine is formally retired.


## 1. Contracts: AgentStatus and ContentEvent

- [x] 1.1 Add `AgentStatus` enum (`Idle`, `Working`, `WaitingInput`, `Done`) to `crates/contracts/src/lib.rs` with `Serialize`, `Deserialize`, `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`
- [x] 1.2 Add `ContentEvent` struct (`kind: String`, `tool_name: String`, `tool_input: serde_json::Value`) to `crates/contracts/src/lib.rs` with `Serialize`, `Deserialize`, `Debug`, `Clone`, `PartialEq`
- [x] 1.3 Add round-trip serialization tests for `AgentStatus` and `ContentEvent` in `crates/contracts/tests/wire.rs`

## 2. Orchestrator: agent module

- [x] 2.1 Create `crates/orchestrator/src/agent/mod.rs` and declare the module in `lib.rs`
- [x] 2.2 Add `AgentDefinition` struct in `crates/orchestrator/src/agent/definition.rs` (binary name, launch args template, CLI version range, interrupt sequence, binary-resolution policy); add a `AGENT_DEF` constant wired to the coding-agent CLI defaults
- [x] 2.3 Implement `hook_to_status(event: &HookEvent) -> AgentStatus` in `crates/orchestrator/src/agent/parse.rs`; unit-test all six `HookKind` variants
- [x] 2.4 Implement `hook_to_content(event: &HookEvent) -> Option<ContentEvent>` in the same file; unit-test `PostToolUse` produces content, all other variants return `None`

## 3. Orchestrator: hook setup in Rust

- [x] 3.1 Create `crates/orchestrator/src/agent/setup.rs` with `install(agent_home: &Path, notify_command: &str) -> Result<()>` and `uninstall(agent_home: &Path) -> Result<()>`; use marker constant `"tillerd-notify"`, same six hook events and `PostToolUse` matcher as the TS reference
- [x] 3.2 Implement atomic write with single overwritten `.bak` backup (read → copy to `settings.json.bak` → modify → write atomically); one backup file, overwritten on each install (design decision #8)
- [x] 3.3 Unit-test idempotent install (no-op on second call), install with pre-existing user hooks preserved, legacy-entry migration, and uninstall restores file
- [x] 3.4 Unit-test uninstall when no hooks present is a no-op

## 4. Orchestrator: SurfaceEventSink extension

- [x] 4.1 Add `on_content(&self, surface: &SurfaceId, event: &ContentEvent)` to `SurfaceEventSink` in `surface/mod.rs` with a default no-op body
- [x] 4.2 Add `on_error(&self, surface: &SurfaceId, reason: &str)` to `SurfaceEventSink` with a default no-op body
- [x] 4.3 Update `NullSink` and all test doubles in `surface/runtime.rs` tests if they explicitly implement the trait (compile-check only — default bodies cover most)

## 5. Orchestrator: open_agent and gate subscription task

- [x] 5.1 Add an `AgentProxy` entry type (alongside the existing terminal proxy) in `surface/runtime.rs` that holds the gate-drain task `JoinHandle`
- [x] 5.2 Implement `SurfaceRuntime::open_agent(surface_id, agent_home, cols, rows, cwd)` that: runs `setup::install`, opens a gate subscription connection (using `gate_client::encode_subscribe_preamble`), spawns the gate-drain task storing its `JoinHandle`, then calls `open_terminal` logic for the daemon spawn
- [x] 5.3 Implement the gate-drain `tokio::task`: loop decoding `SubscriptionFrame`s; on `Event`, call `hook_to_status` → `sink.on_status` and `hook_to_content` → optional `sink.on_content`; on `Error`, call `sink.on_error` and exit; task exits cleanly when the handle is aborted (stream closes)
- [x] 5.4 Extend `SurfaceRuntime::remove` to abort the gate-drain task `JoinHandle` (if present) and call `setup::uninstall` for agent surfaces; confirm idempotent on second remove call
- [x] 5.5 Unit-test `open_agent`: fake gate server (Unix socket) sends `ready` then two events; assert `on_status` called twice and `on_content` called once (for the `PostToolUse` event)
- [x] 5.6 Unit-test gate error frame: fake gate sends `error` after `ready`; assert `on_error` called and task exits

## 6. SurfaceApi: create_agent_surface

- [x] 6.1 Add `SurfaceApi::create_agent_surface(surface_id, cols, rows, cwd)` to `surface/api.rs` that creates a session + surface row (`SurfaceKind::Agent`), calls `SurfaceRuntime::open_agent`, and returns the `SurfaceId`
- [x] 6.2 Unit-test `create_agent_surface`: verify surface row has `kind = Agent`, proxy count is 1, gate subscription task is running

## 7. Tauri host: event bindings

- [x] 7.1 Add `on_content` to the Tauri `EventSink` implementation: emit a `surface:content` Tauri event with `surface_id` and `ContentEvent` payload
- [x] 7.2 Add `on_error` to the Tauri `EventSink` implementation: emit a `surface:error` Tauri event with `surface_id` and `reason`
- [x] 7.3 Add `create_agent_surface` Tauri command that delegates to `SurfaceApi::create_agent_surface`

## 8. UI: AgentPane component

- [x] 8.1 Create `apps/ui/app/components/AgentPane.tsx`: vertical split layout with embedded `TerminalPane` (top, ~80%) and content list (bottom, ~20%); status badge overlay in top-right corner
- [x] 8.2 Implement status badge rendering for `IDLE`, `WORKING`, `WAITING_INPUT`, `DONE`, and `crashed` states; subscribe to `surface:status` Tauri events filtered by `surface_id`
- [x] 8.3 Implement content list: subscribe to `surface:content` events, append `tool_use` entries (tool name + tool input), cap at 500 entries (drop oldest)
- [x] 8.4 Implement failure state: subscribe to `surface:error` events, display dismissible banner above content list showing raw `reason` string with a `[Dismiss]` button; terminal pane remains visible (design decision #9)
- [x] 8.5 Add `{ type: "agent"; sessionId: string | null }` to the `PanelContent` union in `lib/panelTree.ts`; update `renderContent()` in `AppShell.tsx` to render `AgentPane` for `type = "agent"` surfaces

## 9. Integration and validation

- [x] 9.1 Run `cargo test -p contracts` — confirm new types round-trip
- [x] 9.2 Run `cargo test -p orchestrator` — confirm all new and existing tests pass
- [x] 9.3 Run `cargo clippy -p contracts -p orchestrator -- -D warnings`
- [x] 9.4 Run `cargo fmt --check -p contracts -p orchestrator`
- [x] 9.5 Run `bun run check` (turbo) — confirm UI type-checks and no regressions in TS packages
- [ ] 9.6 Manual smoke test — DEFERRED to 0.1.6 (needs the GUI driving harness; the desktop GUI is not drivable in 0.0.x): open an agent surface, verify the status badge transitions through `IDLE → WORKING → IDLE` on a single agent turn, and at least one content entry appears

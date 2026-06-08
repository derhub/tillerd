# Build Plan: composable-standalone-tools-architecture

Generated from the read-only audit workflow `composable-tools-build-plan-audit` (9 agents, 2026-06-07). TDD-first: build each unit red -> green -> refactor. Implement inline on the main thread, one commit per `tasks.md` item, local only (no push). Full per-scenario test breakdown lives in each `specs/*/spec.md`; this file is the build order, the consolidated test inventory, and per-capability layout.

## Build order

**1. contracts-rs** — tasks 1.1, 1.2
Foundational wire types (HookEvent/HookKind typed payload, SessionId, CorrelationId, subscription + tool-route shapes, camelCase) plus the TS mirror and golden cross-check; every other crate depends on this error/wire taxonomy, so it lands first. (ADR tasks 0.1-0.3 already done.)

**2. service-host** — tasks 2.1, 2.2, 2.3, 2.4
The run-me lifecycle library (deterministic dir/manifest/socket paths with base-dir override, atomic temp+rename manifest, escalating signal shutdown, unauthenticated liveness, host::run entry); daemon and gate later migrate onto it, so it must exist before they do.

**3. process-launch** — tasks 2.5, 2.6
The run-others library (adopt-or-spawn with stale-manifest overwrite and wait-until-reachable, spawn_fields_differ, bounded-backoff restart); depends on contracts-rs error taxonomy; orchestrators and the tool gateway use it to launch backends. Independent of service-host but ships alongside it.

**4. rust-pty-daemon** — tasks 3.1, 3.2, 3.3, 3.4, 3.5
Strip hook ingress (hooks.sock, relay_hook, negotiated hook capability) so the daemon is pty-only and consumer-oblivious; expose the versioned session-event subscription as its sole surface; migrate onto service-host; publish daemon-pty-client and align the TS proxy (remove the hook-frame branch). Depends on contracts-rs + service-host.

**5. agent-adapter** — tasks 4.1, 4.2, 4.3, 4.4
Define the AgentAdapter trait (parse_hook) as a v1 module in the gate binary producing the canonical HookEvent (one unit test per event type), then retire the TS parseHook/parseTranscriptEntry/transcriptPath and reduce the SDK AgentDefinition to declarative config. The gate's Normalize middleware injects this adapter, so it precedes gate wiring.

**6. gate** — tasks 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 5.11, 5.12, 5.13, 5.14
Build the middleware framework (Kind/Ctx/Outbound/Reject/Flow/Next, seq/par) then each middleware (Auth constant-time, Observe around, Normalize via injected adapter, FanOut, PassThrough), the router with globals [Observe outermost, Auth], hook + tool + admin + health endpoints with face isolation and loopback-only binds, the versioned bounded drop-oldest subscription wire, correlation-id propagation, then migrate onto service-host and finish with router integration tests. Depends on contracts-rs, service-host, and the injected adapter.

**7. engram-memory** — tasks 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 6.9
Confirm the engram library is daemon-free, build gate-client (thin wire-decode lib, zero engram dep) tested against wire fixtures, define the HookSource port with a gate-subscription adapter and a stub source, map canonical HookEvent to chunks, add the durable idempotent capture-queue with commit-then-enqueue and a proactive background worker, make recall dual-mode (standalone MCP or gateway backend) keeping the loopback viewer, then remove memory's own POST /hook ingress. Depends on contracts-rs and an operational gate.

**8. tool-composition** — tasks 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7
Wire the thin tool gateway (adopt service-host, forward tool inbounds to gate.handle observe-pass-through, fail-open if gate unreachable, register memory as an ordinary backend, spawn only external backends) and the orchestrators (server/desktop/cli bootstrap tools via process-launch, no peer-spawning; mint+register session with the gate BEFORE spawn, inject gate URL/session/token into the daemon env; deregister on session exit; cli installer targets the gate; server hosts the gate subscription client feeding the engine). Depends on all prior tools.

**9. composition-verification** — tasks 8.1, 8.2, 8.3, 8.4, 8.5
Cross-cutting verification: every specs/\* scenario has a test; the four deployment slices run (memory-only, gateway-only, PTY-only, full); observability traces one correlationId across daemon to gate to gateway to memory; dependency directions hold (no tool to tool, no tool to orchestrator, only daemon-pty-client knows the PTY wire); coordinate ordering so this change lands before daemon-upgrade-drain-restart.

## Cross-cutting risks

- Wire-contract drift: HookEvent/HookKind/CorrelationId must encode identically in contracts-rs and @athing/sdk; the golden cross-check (1.2) is the single guard and must run on every change touching the shape; the subscription wire (5.9) and tool-route shapes share the same risk.
- Correlation-id propagation is end-to-end: assign-once on entry, never reassign, carry through Ctx into the emitted HookEvent and every observability record, and preserve across the daemon to gate to gateway to memory hops (5.10, 8.3). A single reassignment silently breaks the whole debugging trace.
- Constant-time token compare must be used at every authenticated surface (service-host liveness is the exception): gate Auth (5.2), admin surface, and the per-session token mint/register/deregister lifecycle (7.4/7.5). Auth rejection latency must be indistinguishable from a valid path; reuse the daemon's constant_time_eq reference and unit-test for no data-dependent branches.
- Manifest/socket atomicity and stale-resource recovery span service-host (2.1-2.2), process-launch (2.5), and orchestrator teardown (7.5): temp+rename must hold on macOS and Linux, dead sockets must be removed before rebinding, and adopt-or-spawn must recover from a stale manifest naming a dead pid. A crashed tool must not strand a stale manifest that blocks the next launch.
- Fire-and-forget hook ingestion plus bounded drop-oldest delivery must never block the poster: gate returns 200 OK before fan-out (5.8/5.9), drops OLDEST (FIFO) not newest, counts and logs drops; memory's capture queue mirrors this with commit-then-enqueue and a non-blocking worker (6.5/6.6). Reversing the drop direction loses the latest hooks with no audit trail.
- Session-registration ordering is a hard race: the orchestrator must register (session, token, allow-policy) with the gate admin surface BEFORE spawning the agent, else an early hook hits Auth before the token is valid and is rejected 403 (design D7 / lines 282-283, task 7.4). Deregistration on session exit must then make late hooks fail auth (7.5).
- Graceful-degradation / standalone operability must be true for every tool, not asserted once: each tool runs with no peers and degrades rather than crashes (gate unreachable -> tool gateway fail-open 7.1; daemon unavailable -> engine degrades; gate unreachable -> memory degrades). Testing absent peers needs a stub/fake source per tool (memory's stub HookSource), so the port seams must exist before composition tests.
- Dependency-direction enforcement is structural, not just tested: no tool imports another tool's internals, no tool depends on an orchestrator, only daemon-pty-client knows the PTY wire, gate-client carries zero engram dependency (8.4). Enforce via Cargo dependency edges (the DAG) in addition to the wire/contract tests.
- Coordinated breaking deployment: removing daemon hook ingress (3.1) and memory's POST /hook (6.9) is a pre-v1 breaking change to the daemon wire and client protocol; all clients must re-point at the gate simultaneously and bump/validate the wire version (3.2). Land this change before daemon-upgrade-drain-restart (8.5) since both touch the daemon spec.
- service-host abstraction boundary must stay lifecycle+filesystem only and never leak transport/protocol; each tool keeps its own wire (PTY binary framing, gate HTTP/IPC, gateway). If service-host leaks transport, every adopting tool (daemon 3.3, gate 5.13, gateway 7.1, memory) couples on it.

## Open questions (settle before/at implementation)

- Canonical HookEvent typed payload shape: the exact per-HookKind fields (must cover prompt content, tool name/input/response, turn index) are flagged 'settle at implementation' in design Open Questions but block contracts-rs 1.1, the sdk mirror 1.2, the v1 parse_hook 4.1, and capture mapping 6.4. Pin the payload schema before writing 1.1 so the golden cross-check and adapter tests target a fixed shape.
- Per-session channel capacity / drop threshold: the bounded delivery cap is open (design Open Questions); too small drops under normal load, too large bloats memory with many sessions. Settle a default (with a config comment) before gate 5.9 so the drop-oldest tests assert against a known threshold.
- Matching-version semantics for adopt-or-spawn (2.5): the spec says 'matching-version' but does not define exact-match vs semver-range comparison. Decide exact-match-initially with caller-controlled policy, or a range, before writing adopt validation tests (audit-flagged undefined).
- Gate policy file on-disk format: the session allow-policy store format is open (design Open Questions) and is needed by gate Auth (5.2), the admin register/deregister surface (5.12), and orchestrator registration (7.4). Settle before the admin surface and Auth integration tests.
- Dual-mode detection mechanism for memory (6.7): how the tool distinguishes 'engram serve' (viewer only) vs 'engram mcp' (standalone stdio) vs orchestrator-launched composed (env var vs flag vs no-args). Ambiguous detection runs the wrong startup path; pin the contract before recall dual-mode tests.
- ATHING_DIR relative-path resolution must match TypeScript path.resolve() semantics against cwd (service-host 2.1 risk): confirm whether full parity with the existing TS supervisor resolution is required, since process-launch and orchestrators must resolve identically to avoid instance collisions.
- spawn_fields_differ field set (2.6): which fields are spawn-affecting (binary, args, spawn-affecting env) vs non-affecting (logging level, observer/metadata) must be enumerated before the differ tests, since the TS reference does not yet exercise this logic.
- Tool gateway fail-open vs fail-closed when the gate is unreachable (7.1): design says fail-open (log + proceed) for the observe-only v1 PassThrough, but confirm this is intended for the tool route specifically and document that it changes once policy middleware (validate/firewall/redaction) lands.
- Versioned subscription-wire handshake/negotiation (5.9 + 3.2): confirm a single version-negotiation mechanism is shared between the daemon session-event wire and the gate hook-subscription wire, or that they are independently versioned, before mirroring shapes into contracts-rs and sdk.

## Test inventory

### contracts-rs (3)

- hook_event encodes camelCase per HookKind variant
- golden cross-check rust HookEvent encoding equals @athing/sdk encoding
- subscription and tool-route message shapes round-trip

### service-host (21)

- host_invokes_serve_behavior_after_resource_setup
- host_resolves_base_directory_before_manifest_write
- host_installs_signal_handlers_for_graceful_termination
- manifest_path_deterministic_from_base_dir
- socket_path_deterministic_from_base_dir
- all_paths_rooted_at_base_directory
- base_dir_override_honored_via_athing_dir_env
- base_dir_override_absolute_path_unchanged
- base_dir_override_relative_path_resolved_against_cwd
- manifest_write_uses_temp_file_and_rename
- manifest_carries_pid_and_version
- manifest_never_partial_on_interrupt
- manifest_removed_after_graceful_shutdown
- manifest_persists_after_signal_before_handler_runs
- manifest_cleanup_survives_sigkill
- sigterm_triggers_graceful_shutdown_sequence
- escalation_to_sigkill_after_grace_period
- no_orphaned_children_after_shutdown
- liveness_endpoint_responds_without_auth
- liveness_returns_version_and_reachability
- liveness_rejects_invalid_requests

### process-launch (18)

- adopt_does_not_spawn_second_instance_when_live_matching_exists
- adopt_connects_to_existing_socket_without_restart
- adopt_validates_matching_version_before_connecting
- spawn_launches_new_process_when_socket_missing
- spawn_waits_until_socket_reachable
- spawn_honors_startup_timeout_on_unresponsive_binary
- spawn_overwrites_stale_manifest_on_dead_pid
- spawn_cleans_dead_socket_before_binding
- spawn_proceeds_despite_stale_manifest_pid
- spawn_fields_differ_detects_command_change
- spawn_fields_differ_detects_args_change
- spawn_fields_differ_detects_env_var_change
- spawn_fields_differ_ignores_metadata_change
- spawn_fields_differ_ignores_logging_level_change
- spawn_fields_differ_ignores_observer_field_change
- restart_backoff_does_not_exceed_cap
- restart_attempts_respect_max_attempts
- restart_backoff_uses_exponential_strategy

### rust-pty-daemon (5)

- daemon_requires_no_modification_when_hook_capable_consumer_connects
- daemon_requires_no_modification_when_consumer_disconnects
- daemon_ignores_hook_capable_flag_entirely
- daemon_serves_session_lifecycle_to_any_subscriber_regardless_of_capability
- daemon_only_deployment_serves_plain_terminal_with_no_hook_ingress

### daemon-session-subscription (8)

- test_daemon_relays_bytes_to_unaware_subscriber_without_carrying_consumer_knowledge
- test_adding_new_subscriber_type_requires_no_daemon_modification
- test_subscriber_receives_session_output_stream_keyed_by_id
- test_subscriber_receives_session_lifecycle_events
- test_client_advertising_versions_daemon_selects_highest_mutually_supported
- test_wire_shape_change_originates_from_versioned_definition_mirrored_to_all_surfaces
- test_daemon_public_surface_contains_no_hook_ingress_endpoint
- test_hook_endpoint_removed_from_hello_ack_capabilities

### agent-adapter (2)

- parse_hook produces expected canonical HookEvent per raw event type (one per variant)
- it rejects with Invalid error on parse failure

### gate (61)

- it routes a hook inbound through Observe middleware then Auth middleware
- it routes a tool inbound through Observe middleware then Auth middleware
- it runs hook-route-specific middleware only for Kind::Hook inbounds
- it runs tool-route-specific middleware only for Kind::ToolCall and Kind::ToolResult inbounds
- it stops and returns rejection from Auth when token is invalid or missing
- it does not call later middleware when Auth rejects
- it records the rejection in Observe before returning to caller
- it runs seq() middleware in order and stops on first Reject or Done
- it runs par() middleware concurrently and joins all results
- it propagates context mutations from TRANSFORM middleware to next in seq chain
- it returns Outbound::Forward(bytes) for a valid ToolCall inbound
- it returns Outbound::Forward(bytes) for a valid ToolResult inbound
- it includes original body bytes unchanged in Forward response
- it contains no MCP protocol parsing or client code
- it holds no backend registry or tool definitions
- it speaks only the generic Inbound/Outbound contract
- it emits one observability record per inbound handled
- it includes session id and correlation id on every record
- it includes kind field showing Hook or ToolCall or ToolResult
- it includes outcome field showing accepted or rejected with reason
- it calls parse_hook on raw hook body and sets ctx.event
- it publishes resulting HookEvent to all subscribers of the session
- it delivers same event to multiple concurrent subscribers
- it accepts hook POST and returns 200 OK before fanning out to any subscriber
- it does not wait for subscriber acknowledgment
- it does not block on full subscriber queues
- it delivers identical HookEvent to N concurrent subscribers of same session
- it does not drop or modify event when copying to multiple subscribers
- it calls the injected AgentAdapter.parse_hook() method
- it rejects with Invalid error on parse failure
- it uses parsed HookEvent from adapter, not hard-coded logic
- it contains no agent-specific parsing logic or enum variants
- it depends only on AgentAdapter trait interface
- it compiles and routes unchanged when adapter implementation swapped
- it delivers HookEvent to consumer subscribed to that session
- it does not deliver events for different sessions to this consumer
- it closes subscription stream when session ends
- it releases the bounded per-session channel
- it stops accepting new hooks for deregistered session
- it drops oldest pending event when per-session buffer is full
- it increments dropped count and logs the drop
- it accepts new hook POST and continues without waiting for subscriber
- it carries caller-supplied correlation id through Observe, Normalize, FanOut
- it places same correlation id on emitted HookEvent
- it binds same correlation id in observability record
- it generates fresh UUID when caller supplies no correlation id
- it uses same assigned id consistently for that message's processing
- it places assigned id on HookEvent and observability record
- tool route POST to hook endpoint returns 401 or 403 Unauthorized
- tool route cannot call FanOut middleware or publish events
- tool route middleware chain is isolated from hook route chain
- hook endpoint cannot call session-register or session-deregister endpoints
- tool route cannot call session-register or session-deregister endpoints
- admin surface requires separate authentication
- it binds all HTTP listeners to 127.0.0.1 only not 0.0.0.0
- it binds hook endpoint to Unix socket on loopback-only path
- it binds tool route IPC to Unix socket on loopback-only path
- it accepts GET /health without token and returns liveness + version
- it rejects non-health endpoint without valid token header
- it validates token against session store for hook and tool routes
- hook endpoint enforces transport-level max-body-size cap (OOM guard)

### engram-capture (19)

- capture_prompt_fires_when_user_submits_via_hook_source
- capture_prompt_rejects_empty_content_without_creating_chunk
- capture_prompt_idempotent_under_duplicate_hook_fire
- capture_tool_fires_when_post_tool_use_hook_arrives
- capture_tool_skips_low_value_tools_on_skip_list
- capture_tool_stringifies_response_and_structures_input
- capture_tool_rejects_when_tool_name_is_on_skip_list
- hook_source_adapter_enqueues_asynchronously_without_waiting_for_queue_drain
- hook_ingestion_does_not_block_on_embedding_computation
- stub_hook_source_produces_same_chunks_as_gate_subscription
- hook_source_port_allows_injection_of_test_stub_in_unit_tests
- ingest_commits_chunk_to_db_before_enqueuing_embedding_request
- embedding_request_enqueued_even_if_worker_not_running
- capture_queue_persists_pending_requests_across_restart
- pending_requests_remain_in_queue_after_memory_tool_restarts
- background_worker_drains_queue_without_waiting_for_recall_request
- embedding_requests_processed_after_configured_drain_interval
- duplicate_embedding_request_for_same_chunk_idempotent
- content_hash_dedup_prevents_duplicate_chunks_on_retry

### gate-client (1)

- subscribe decode matches hook-subscription wire fixtures

### engram-recall (11)

- test_viewer_binds_loopback_only_in_standalone_mode
- test_viewer_binds_loopback_only_when_composed
- test_standalone_mcp_stdio_server_is_reachable
- test_standalone_recall_tool_works_without_gateway
- test_standalone_mcp_initialization_succeeds
- test_gateway_routes_recall_to_memory_backend
- test_memory_recall_backend_behaves_identically_in_composed_mode
- test_gateway_applies_no_special_casing_to_memory_backend
- test_memory_tool_startup_accepts_dual_mode_flag
- test_viewer_remains_accessible_in_both_modes
- memory_only_deployment_subscribes_to_gate_without_daemon_or_gateway

### tool-composition (25)

- tool_runs_standalone_without_daemon
- tool_runs_standalone_without_gate
- tool_runs_standalone_without_gateway
- memory_degrades_when_gate_unreachable
- tool_gateway_degrades_when_gate_unreachable
- engine_degrades_when_daemon_unavailable
- orchestrator_launches_selected_tools_via_process_launch
- orchestrator_creates_separate_processes_for_each_tool
- daemon_does_not_spawn_gateway
- memory_does_not_spawn_daemon
- gateway_does_not_spawn_memory
- memory_depends_only_on_contracts_rs_and_gate_client
- tool_gateway_depends_only_on_contracts_rs
- no_tool_imports_another_tool_internal_modules
- daemon_has_no_orchestrator_dependency
- memory_has_no_orchestrator_dependency
- gateway_has_no_orchestrator_dependency
- memory_hook_source_uses_wired_port_not_hardcoded_daemon
- memory_consume_gate_subscription_when_gate_wired
- memory_consume_stub_source_when_testing
- cli_hook_installer_targets_gate_when_gate_wired
- hook_event_flows_to_correct_ingress_based_composition
- orchestrator_passes_gate_url_to_daemon_env
- orchestrator_registers_session_with_gate_before_spawn
- orchestrator_deregisters_session_on_daemon_exit_late_hooks_fail_auth

### composition-verification (4)

- every specs scenario maps to at least one passing test (8.1)
- four deployment slices boot - memory-only gateway-only pty-only full (8.2)
- one correlationId traces an action across daemon to gate to gateway to memory (8.3)
- dependency graph has no tool-to-tool and no tool-to-orchestrator edges (8.4)

## Per-capability layout

### service-host — `packages/service-host`

- Depends on: contracts-rs (foundational Rust crate for wire types; service-host must exist before other tools adopt it)
- New modules:
  - `packages/service-host/Cargo.toml`
  - `packages/service-host/src/lib.rs`
  - `packages/service-host/src/manifest.rs`
  - `packages/service-host/src/paths.rs`
  - `packages/service-host/src/signals.rs`
  - `packages/service-host/src/shutdown.rs`
  - `packages/service-host/src/probe.rs`
  - `packages/service-host/tests/integration_tests.rs`
- Existing touchpoints:
  - packages/daemon-pty/src/manifest.rs:1-86 (extract manifest write/read/remove patterns)
  - packages/daemon-pty/src/main.rs:68-163 (extract signal handling and graceful shutdown patterns)
  - packages/daemon-pty/src/signals.rs:1-328 (signal table and category mapping, reusable)
  - packages/daemon-pty/src/resolve.rs (extract path resolution patterns)
  - packages/daemon-pty/src/hook_ingress.rs:67-76 (constant-time comparison pattern from token auth)

### process-launch — `packages/process-launch`

- Depends on: contracts-rs (new foundational Rust crate for wire types and shared contracts — must land first; process-launch depends on its error taxonomy); service-host (new Rust library for lifecycle concerns; process-launch is independent but both ship in the same change)
- New modules:
  - `packages/process-launch/src/lib.rs (root module exposing adopt_or_spawn, spawn_fields_differ, restart_backoff, and types)`
  - `packages/process-launch/src/manifest.rs (read/write manifest with pid + version; atomic rename; path resolution honoring ATHING_DIR)`
  - `packages/process-launch/src/adopt.rs (connect to live matching-version instance by reading manifest, checking pid alive, verifying socket reachable)`
  - `packages/process-launch/src/spawn.rs (spawn process, wait for reachability, overwrite stale manifest, handle timeout/failure)`
  - `packages/process-launch/src/diffing.rs (spawn_fields_differ: compare only fields affecting process spawn — binary, args, spawn-affecting env)`
  - `packages/process-launch/src/backoff.rs (exponential backoff strategy with cap; attempt counter; sleep logic)`
  - `packages/process-launch/src/error.rs (typed errors: BinaryNotFound, SpawnFailed, Timeout, VersionMismatch, SocketUnresponsive per ADR-0007)`
  - `packages/process-launch/Cargo.toml (dependencies: serde, serde_json, thiserror, tokio, nix for signal/process)`
  - `packages/process-launch/src/manifest.rs [tests] (unit tests for path resolution, atomic write, read roundtrip)`
- Existing touchpoints:
  - packages/daemon-pty/src/manifest.rs:29-46 (ManifestData shape and Manifest::read/write; process-launch SHALL mirror this for compatibility)
  - packages/platform-bun/src/supervisor.ts:51-96 (adoptOrSpawn TypeScript reference implementation; Rust process-launch is the Rust equivalent)
  - packages/platform-bun/src/supervisor.ts:31-38 (isAlive and readManifest helpers; process-launch SHALL reimplement for Rust)
  - apps/server/src/index.ts:13 (imports adoptOrSpawn; server WILL replace with Rust process-launch)
  - packages/daemon-pty/src/main.rs:19-46 (athing_dir path resolution and ATHING_DIR env handling; process-launch SHALL match)
  - packages/daemon-pty/src/main.rs:156-160 (socket poll-and-write manifest pattern; spawn MUST wait for socket reachability before return)
  - packages/daemon-pty/src/resolve.rs (resolve_command logic for binary lookup; process-launch may reuse or wrap)

### rust-pty-daemon — `packages/daemon-pty`

- Depends on: contracts-rs (new foundational Rust crate for wire types); service-host (task 2.1-2.6 lifecycle library)
- Existing touchpoints:
  - packages/daemon-pty/src/main.rs:6
  - packages/daemon-pty/src/main.rs:19
  - packages/daemon-pty/src/main.rs:99
  - packages/daemon-pty/src/server.rs:6
  - packages/daemon-pty/src/server.rs:7
  - packages/daemon-pty/src/server.rs:38
  - packages/daemon-pty/src/server.rs:62-63
  - packages/daemon-pty/src/server.rs:80-81
  - packages/daemon-pty/src/server.rs:85-88
  - packages/daemon-pty/src/server.rs:107-108
  - packages/daemon-pty/src/server.rs:227
  - packages/daemon-pty/src/server.rs:307-315
  - packages/daemon-pty/src/server.rs:321
  - packages/daemon-pty/src/server.rs:471-478
  - packages/daemon-pty/src/server.rs:480-496
  - packages/daemon-pty/src/server.rs:520
  - packages/daemon-pty/src/manifest.rs:2
  - packages/daemon-pty/src/manifest.rs:35-36
  - packages/daemon-pty/src/manifest.rs:122
  - packages/daemon-pty/src/hook_ingress.rs (entire file - delete)

### daemon-session-subscription — `packages/daemon-pty`

- Depends on: contracts-rs (foundational wire types); service-host (task 2.1-2.4); process-launch (task 2.5-2.6); rust-pty-daemon (spec modification)
- New modules:
  - `packages/contracts-rs/src/lib.rs`
  - `packages/contracts-rs/src/wire/mod.rs`
  - `packages/contracts-rs/src/wire/hook_event.rs`
  - `packages/contracts-rs/src/wire/session_event.rs`
  - `packages/contracts-rs/src/wire/correlation.rs`
  - `packages/daemon-pty-client/src/lib.rs`
  - `packages/daemon-pty-client/src/decoder.rs`
- Existing touchpoints:
  - packages/daemon-pty/src/main.rs:6
  - packages/daemon-pty/src/main.rs:19
  - packages/daemon-pty/src/main.rs:99
  - packages/daemon-pty/src/main.rs:102
  - packages/daemon-pty/src/server.rs:6
  - packages/daemon-pty/src/server.rs:38
  - packages/daemon-pty/src/server.rs:62-63
  - packages/daemon-pty/src/server.rs:81
  - packages/daemon-pty/src/server.rs:307-315
  - packages/daemon-pty/src/server.rs:321
  - packages/daemon-pty/src/server.rs:471-478
  - packages/daemon-pty/src/server.rs:480-496
  - packages/daemon-pty/src/server.rs:498-505
  - packages/daemon-pty/src/server.rs:520
  - packages/daemon-pty/src/server.rs:641
  - packages/daemon-pty/src/manifest.rs:35
  - packages/daemon-pty/src/manifest.rs:36
  - packages/daemon-pty/src/messages.rs:4
  - packages/daemon-pty/src/messages.rs:321
  - packages/daemon-pty/Cargo.toml
  - packages/sdk/src/protocol/messages.ts:4
  - packages/sdk/src/protocol/messages.ts:12
  - packages/sdk/src/protocol/messages.ts:28

### gate — `apps/gate (new gate binary)`

- Depends on: contracts-rs (task 1.1 and 1.2: must define HookEvent, HookKind, SessionId, CorrelationId, subscription wire, tool-route message shapes before gate can use them); service-host library (task 2.1-2.4: gate migrates onto service-host for lifecycle, manifest, socket, signals, liveness in task 5.13)
- New modules:
  - `apps/gate/src/lib.rs (middleware framework: Middleware trait, Router, Ctx, Outbound, Reject, Flow, Next, seq/par combinators, noop/spy test helpers)`
  - `apps/gate/src/middleware/auth.rs (constant-time per-session token comparison against policy handle)`
  - `apps/gate/src/middleware/observe.rs (around-shape middleware: bind session id + correlation id + component, emit correlation-bound structured record with Resource identity, ts, kind, eventType, outcome, latencyMs, fanoutN, droppedN)`
  - `apps/gate/src/middleware/normalize.rs (call injected AgentAdapter, set ctx.event, reject on parse error)`
  - `apps/gate/src/middleware/fanout.rs (terminal middleware: publish canonical HookEvent to per-session subscribers via par combinator)`
  - `apps/gate/src/middleware/passthrough.rs (terminal middleware for tool route: return Forward outbound unchanged)`
  - `apps/gate/src/router.rs (dispatch by Kind, wire globals [Observe outermost then Auth], Hook and Tool routes, gate.handle(inbound) -> Flow)`
  - `apps/gate/src/agent_adapter.rs (AgentAdapter trait: parse_hook method; v1 module-local implementation producing canonical HookEvent from raw agent format)`
  - `apps/gate/src/endpoint/hook.rs (loopback HTTP POST endpoint, per-session token auth, transport-level max-body-size cap, fire-and-forget response)`
  - `apps/gate/src/endpoint/tool.rs (tool route entry over local IPC, length-prefixed loopback framing for ToolCall/ToolResult inbounds)`
  - `apps/gate/src/endpoint/health.rs (unauthenticated health endpoint: liveness + version)`
  - `apps/gate/src/endpoint/admin.rs (session registration/deregistration surface, separate authenticated surface, isolated from hook and tool routes)`
  - `apps/gate/src/subscription.rs (versioned wire matching contracts-rs spec, per-session bounded broadcast channels, drop-oldest policy with drop count + logging)`
  - `apps/gate/src/bin/main.rs (service-host entry point, spawns all endpoints, wires router, graceful shutdown, liveness probe)`
- Existing touchpoints:
  - packages/daemon-pty/src/hook_ingress.rs: entire module removed (tasks 3.1 - hook ingress moves to gate)
  - packages/daemon-pty/src/server.rs:10-11: remove HookIngress member from Daemon struct
  - packages/daemon-pty/src/server.rs:37-39: remove hook_ingress Connection union variant
  - packages/daemon-pty/src/main.rs:6,19,99: remove hook_ingress module and related initialization
  - packages/daemon-pty/src/manifest.rs:35: remove hooks_sock path (gate owns hook endpoint)
  - packages/sdk/src/types/adapter.ts:31-33: remove parseHook, transcriptPath, parseTranscriptEntry from AgentDefinition interface (task 4.4)
  - packages/sdk/src/types/events.ts:34-38: replace with canonical HookEvent mirroring contracts-rs shape including correlationId and typed per-type payload (task 1.2)
  - apps/engram-rs/src/main.rs: POST /hook handler removal (task 6.9)
  - apps/engram-rs/src/lib.rs: add HookSource port with gate-subscription adapter (task 6.3), add durable capture-queue table and worker (task 6.5-6.6)
  - apps/mcp-gateway-rs/src/bin/gateway.rs: send ToolCall/ToolResult inbounds to gate.handle() and adopt service-host (task 7.1)
  - apps/server/src/\*.ts: add TS gate-client subscription (task 7.7), engine consumes HookEvent from gate not transcript
  - packages/daemon-pty/src/server.rs:193: remove relay_hook call and session_token/subscribe calls

### engram-capture — `apps/engram-rs`

- Depends on: 1.1 (contracts-rs: HookEvent + HookKind + CorrelationId wire types); 1.2 (@athing/sdk mirror of HookEvent); 5.1-5.14 (gate binary: middleware framework, hook/tool routes, subscription wire, observability); 6.2 (gate-client: Rust lib to decode hook-subscription wire from contracts-rs)
- New modules:
  - `apps/engram-rs/src/hook_source.rs — HookSource port trait + gate-subscription adapter + stub source for tests`
  - `apps/engram-rs/src/capture_queue.rs — durable queue table schema + operations (insert, drain, reclaim stale)`
  - `apps/engram-rs/src/worker.rs — proactive background worker (spawn on init, drain on blocking pool, restart on panic)`
- Existing touchpoints:
  - apps/engram-rs/src/lib.rs:114-119 — Engram struct gains hook_source: Arc<dyn HookSource> field; constructor passes it
  - apps/engram-rs/src/lib.rs:181-183 — ingest() remains synchronous, but enqueues embedding async (returns immediately)
  - apps/engram-rs/src/lib.rs:321-323 — embed_pending() becomes internal; replace with worker drain (no public embed call on hot path)
  - apps/engram-rs/src/store.rs:42-61 — insert_chunk() signature unchanged, idempotent via UNIQUE constraint (already exists)
  - apps/engram-rs/src/schema.sql — add capture_queue table: (id, chunk_id, status, attempts, last_tried, created_at)
  - apps/engram-rs/src/server.rs:1-2 — remove POST /hook handler (ingest_hook function deleted; line 38-47, 57-112)
  - apps/engram-rs/src/server.rs:35-50 — route() function: remove ('POST', '/hook') case; keep ('GET', '/') viewer + health
  - apps/engram-rs/src/main.rs:155-158 — 'serve' command kept for human viewer only (GET / + /healthz); no listen for hooks
  - apps/engram-rs/Cargo.toml — add: tokio (async runtime), contracts-rs (HookEvent type), gate-client (subscription decode)

### engram-recall — `apps/engram-rs`

- Depends on: contracts-rs (wire types HookEvent, SessionId, CorrelationId per task 1.1); service-host (engram binary lifecycle, task 2.1-2.4); gate-client (HookSource gate-subscription adapter, task 6.2); gate (operational before memory can subscribe, task 5.x); engram-capture (task 6.3-6.4 depend on HookSource port being defined here)
- New modules:
  - `apps/engram-rs/src/dual_mode.rs: Dual-mode configuration/initialization (standalone vs composed detection)`
  - `apps/engram-rs/src/mcp_server.rs: Split from existing mcp.rs to separate stdio MCP server logic (standalone-only)`
  - `apps/engram-rs/src/viewer_server.rs: Split from existing server.rs to retain GET / viewer endpoint (both modes)`
  - `Cargo.toml additions: service-host dependency, gate-client dependency`
- Existing touchpoints:
  - apps/engram-rs/src/server.rs:34-50: route() — DELETE POST /hook branch (ingest_hook removal), RETAIN GET / and GET /healthz
  - apps/engram-rs/src/server.rs:59-112: ingest_hook() — DELETE entirely (capture now via HookSource gate subscription per 6.3-6.4)
  - apps/engram-rs/src/server.rs:139-188: serve() — RETAIN for viewer only, rename to serve_viewer()
  - apps/engram-rs/src/main.rs:152-158: 'serve' command — UPDATE to call renamed serve_viewer(), add dual-mode startup
  - apps/engram-rs/src/main.rs:152-154: 'mcp' command — RETAIN for standalone-only mode
  - apps/engram-rs/src/lib.rs:114-152: Engram struct and open() — ADD optional HookSource port dependency (6.3)
  - apps/engram-rs/src/lib.rs:1-17: Module declarations — ADD new dual_mode, mcp_server, viewer_server modules

### tool-composition — `cross-cutting: apps/mcp-gateway-rs + orchestrators (server/desktop/cli)`

- Depends on: service-host (task 2.1-2.4); process-launch (task 2.5-2.6); contracts-rs (task 1.1-1.2); gate (tasks 5.1-5.14); daemon-session-subscription (task 3.1-3.5); engram-capture (task 6.1-6.4 HookSource port); rust-pty-daemon becomes pty-only (task 3.1-3.4)
- Existing touchpoints:
  - packages/daemon-pty/src/hook_ingress.rs (remove hook ingress, keep only PTY)
  - packages/daemon-pty/src/server.rs:1-100 (migrate daemon lifecycle to service-host)
  - packages/daemon-pty/src/manifest.rs (adopt service-host manifest patterns)
  - apps/mcp-gateway-rs/src/lib.rs:1-41 (gateway adopts service-host and sends tool inbounds to gate.handle)
  - apps/mcp-gateway-rs/src/daemon.rs:46+ (gateway lifecycle: adopt service-host)
  - apps/engram-rs/Cargo.toml (memory adopts service-host, adds gate-client dependency)
  - packages/sdk/src/types/adapter.ts:23-34 (drop parseHook, transcriptPath, parseTranscriptEntry from AgentDefinition)
  - apps/server/src/index.ts (server orchestrator: bootstrap tools via process-launch, wire ports)
  - apps/desktop/src-tauri/src/main.rs (desktop orchestrator: bootstrap tools via process-launch, wire ports)
  - apps/cli (cli hook installer: target gate, not daemon)

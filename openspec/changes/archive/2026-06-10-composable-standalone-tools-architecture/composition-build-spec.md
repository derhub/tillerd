# Composition Build Spec (tasks 7.1–7.7)

Synthesized from the `composition-build-spec` design workflow (6 agents, 2026-06-07), grounded in the real gateway/server/cli/desktop/platform-bun/engine code. Cross-cutting integration layer (Rust + TS). Depends on the gate (5.x), memory (6.x), daemon (3.x), process-launch + service-host (2.x). One commit per `tasks.md` item.

## Resolved decisions (accepted from design openDecisions)

- **process-launch boundary:** Rust consumers (mcp-gateway-rs external-backend spawn; desktop `src-tauri`) use the **process-launch crate directly**; TS consumers (server, cli) use a **TS port in `@athing/platform-bun`** (generalize the existing daemon-only `adoptOrSpawn` in `supervisor.ts` into a tool-agnostic launcher). **No** process-launch CLI binary, **no** FFI. Both validated against ONE shared table-driven spec (R3 exact-version adopt, R6 spawn-field set `{command,args,cwd,env[allowlist]}`, R7 ATHING_DIR parity).
- **7.7 client:** a **thin TS gate-client** in `apps/server/src/gate-client.ts` — separate from the Rust `gate-client` (which serves the Rust memory tool, 6.2); not a shared crate. The server is the only TS subscriber; the engine is transport-agnostic (ADR-0005); per-language thin decoders over the versioned, sdk-mirrored wire avoid a tool→tool dep.
- **Env-var names (canonical):** `ATHING_GATE_URL`, **`ATHING_SESSION_ID`**, **`ATHING_SESSION_TOKEN`** — match the shipped `bin/athing-notify` (headers `x-session-id`, `Authorization: Bearer <token>`). tasks.md 7.4 currently says `ATHING_SESSION`/`ATHING_TOKEN`; update its wording to the canonical names. (Supersedes the earlier `ATHING_SESSION` note in memorya-build-spec.)
- **Subscription transport:** loopback Unix socket at the deterministic path `$ATHING_DIR/gate-subscribe.sock` (discovered from `ATHING_DIR`, no extra env var). Distinct from the hook HTTP face.
- **Admin face auth:** a separate startup-provisioned `ATHING_GATE_ADMIN_TOKEN` (registration happens before any session token is trusted), isolated from hook + tool routes.
- **Admin register payload:** orchestrator sends the raw token over the loopback admin face; the **gate hashes/stores the digest at a single site** (keeps constant-time compare + registry consistent).
- **`ATHING_BRIDGE_URL`:** removed from the composed daemon spawn env (daemon hook ingress is gone). Retained ONLY in `bin/athing-notify` as the legacy standalone-without-gate fallback.
- **CorrelationId:** opaque UUID v4, assigned once at the gate Router entry; W3C traceparent deferred.

## Affected packages

- `apps/mcp-gateway-rs` (7.1, 7.2): adopt service-host (replace bespoke daemon.rs lifecycle); local length-prefixed gate IPC client; route ToolCall/ToolResult → `gate.handle()`; **fail-open** (log + forward unchanged) on unreachable/Reject/malformed; register memory as an ordinary backend; spawn only external (non-first-party) backends via the process-launch crate.
- `packages/platform-bun` (7.3, 7.6): generalize `supervisor.ts` into a tool-agnostic TS process-launch port; add `resolveGateUrl()` (env `ATHING_GATE_URL`, else `$ATHING_DIR/gate.url`, else undefined); `buildSetupContext` gains `gateUrl/sessionId/sessionToken`.
- `packages/sdk` (7.6, 7.7): `SetupContext` gains optional `gateUrl/sessionId/sessionToken`; consumes the mirrored `HookEvent` + `HOOK_SUBSCRIPTION_WIRE_VERSION`.
- `packages/adapter-claude-code` (7.6): `setup.install` branches on `ctx.gateUrl` → curl `ATHING_GATE_URL/hook` with `Authorization: Bearer` + `x-session-id`; else legacy `ATHING_BRIDGE_URL`.
- `bin/athing-notify` (7.6): dual-mode — HTTP POST to `ATHING_GATE_URL` with Bearer (exit 0 even when unreachable); retain `ATHING_BRIDGE_URL` unix-socket fallback.
- `apps/cli` (7.3, 7.6): resolve gate url; install gate-targeted hooks; no tool spawning.
- `apps/server` (7.3, 7.4, 7.5, 7.7): orchestrator session lifecycle via the TS port; `gate-client.ts` + gate-bridge feeding the engine.
- `packages/engine` (7.7): proxy/engine consumes the gate HookEvent stream; content derived from `HookEvent.payload`, retiring the transcript reader (ADR-0017 / step 4).
- `apps/desktop` (7.3, 7.4, 7.5): `src-tauri` orchestrator using the process-launch crate directly; desktop does NOT install per-project hooks (cli owns hook install).
- `packages/process-launch`: consumed unchanged as a Cargo dep; its R3/R6/R7 spec is the contract the TS port mirrors.

## Env & wire contracts

- `ATHING_GATE_URL = http://127.0.0.1:<port>` — gate writes `$ATHING_DIR/gate.url` atomically after bind; orchestrator reads it, injects into daemon spawn env (daemon pass-through); composed memory reads it (R5); the agent hook posts to `<url>/hook`.
- `ATHING_SESSION_ID` (UUID v4), `ATHING_SESSION_TOKEN` (32 random bytes hex) — minted by the orchestrator, injected into daemon spawn env; the hook sends `Authorization: Bearer <token>` + `x-session-id`; gate Auth compares constant-time (subtle) vs the registered digest.
- **Register-before-spawn (HARD, R4/D7):** orchestrator mints `{sessionId, token}` → `POST $ATHING_GATE_URL/admin/register {sessionId, token, allowPolicy:All}` (admin face, `ATHING_GATE_ADMIN_TOKEN`) BEFORE spawning the daemon. Then spawn.
- **Deregister-on-exit:** orchestrator observes the daemon PTY session-exit → `DELETE /admin/deregister {sessionId}`; late hooks then fail Auth (Unauthenticated).
- Hook wire (agent→gate): `POST /hook`, `Authorization: Bearer`, `x-session-id`, body = raw agent hook JSON; 200 before fan-out (fire-and-forget; notify exits 0 even if unreachable).
- Tool wire (gateway→gate): length-prefixed loopback IPC (4-byte BE + JSON). As-built shape (see `apps/gate/src/endpoint/tool.rs`): request `{token, inbound: <ToolInbound: {type:ToolCall|ToolResult, payload:{sessionId, correlationId, ...}}>}` → reply `{result: forward|reject}`. Framing reimplemented locally (D9). Fail-open.
- Subscription wire (gate→client): own `HOOK_SUBSCRIPTION_WIRE_VERSION` (R9), mirrored in contracts-rs + sdk; `$ATHING_DIR/gate-subscribe.sock`. As-built shape (see `apps/gate/src/endpoint/subscribe.rs` + `subscription.rs`): request `{sessionId, wireVersion: 1}` → `{frame:"ready", wireVersion:1}` then `{frame:"event", event:<HookEvent>}` / `{frame:"error", message}`; bounded 256 drop-oldest, `ATHING_GATE_QUEUE_CAP`.
- process-launch contract (crate + TS port): adopt on EXACT version (R3); spawn-affecting `{command,args,cwd,env[allowlist={ATHING_DIR,ATHING_GATE_URL,ATHING_SESSION_ID,ATHING_SESSION_TOKEN}]}` (R6); bounded exp backoff (250ms·2^(n-1), 3 attempts).
- Tool manifests under `$ATHING_DIR`: `daemon.json`, `gateway.json`, `memory.json` — `{pid,version}` atomic via service-host.

## Build order (one committable unit each; per-package tests green before commit)

1. **TS process-launch port** [7.3] in `@athing/platform-bun` — generalize `supervisor.ts` to a tool-agnostic launcher (command/args/cwd/env, per-tool manifest path) + `spawn_fields_differ`; validate vs the shared R3/R6/R7 table. Unblocks server + cli.
2. **Tool gateway → service-host + gate routing (fail-open)** [7.1, 7.2] — Rust; replace bespoke daemon.rs lifecycle with service-host; local gate IPC client; route ToolCall/ToolResult → gate.handle; memory as ordinary backend; spawn only external backends. Parallel-able after step 1.
3. **Retarget hook ingress to the gate** [7.6] — SDK SetupContext gains gateUrl/sessionId/sessionToken; platform-bun resolveGateUrl + buildSetupContext; adapter-claude-code install branches on gateUrl (Bearer curl); athing-notify gate HTTP case. Lands before cli.
4. **CLI orchestrator: resolve gate url + install gate hooks** [7.6, 7.3] — thin consumer of step 3; reads ATHING_GATE_URL (env or gate.url); installs gate-targeted hooks; degrade with clear error when no gate url.
5. **Server orchestrator session lifecycle** [7.3, 7.4, 7.5] — mint sessionId+token; register with gate admin BEFORE spawn (HARD); launch via TS port injecting env; observe daemon PTY session-exit → deregister.
6. **Server gate-client + engine HookEvent mapping** [7.7] — thin TS `gate-client.ts` negotiating HOOK_SUBSCRIPTION_WIRE_VERSION, decoding sdk-mirrored HookEvent; gate-bridge → StatusMapper.apply + payload-derived content; engine content from HookEvent.payload (transcript reader retired). Depends on 5.9 + sdk mirror (1.2) + step 4 retirement (ADR-0017).
7. **Desktop Tauri orchestrator** [7.3, 7.4, 7.5] — src-tauri uses the process-launch crate directly; mint+register before spawn; inject env; deregister on exit; no per-project hook install.

## Test inventory (57)

TS process-launch (5): adopts_on_exact_version_match; spawns_and_overwrites_stale_manifest; spawn_fields_differ_table_R6; athing_dir_resolution_parity_R7; bounded_backoff_caps_attempts.
gateway (11): adopts_service_host_lifecycle; uses_service_host_manifest_and_removes_on_clean_stop; uses_service_host_probe_not_separate_listener; uses_service_host_signals_no_bespoke_sigterm; sends_toolcall_toolresult_inbounds_to_gate_handle; fails_open_when_gate_unreachable; logs_warning_and_forwards_unchanged_when_gate_unreachable; gate_malformed_response_log_and_proceed; memory_backend_registered_as_ordinary; spawns_only_external_backends_rejects_first_party_names; binds_loopback_only; correlation_id_preserved_gateway_through_gate_to_backend.
setup/notify/adapter (8): setup_context_carries_optional_gate_url_session_id_token; adapter_install_builds_gate_curl_with_authorization_bearer_when_gate_url_present; adapter_install_falls_back_to_daemon_socket_when_gate_url_absent; installed_hook_includes_authorization_bearer_token; installed_hook_payload_unchanged_same_json_shape; athing_notify_posts_authorization_bearer_to_gate; athing_notify_exits_zero_when_gate_unreachable; resolve_gate_url_prioritizes_env_then_gate_url_file_then_undefined.
cli (4): hook_installer_targets_gate_url_not_daemon_hooks_sock; installer_reads_gate_url_from_base_dir_gate_url_file; hook_installer_degrades_to_daemon_when_gate_absent; hook_installer_idempotent_across_gate_migration (+uninstall symmetric).
server lifecycle (8): mints_session_id_and_per_session_token; registers_session_with_gate_admin_before_spawn; injects_gate_url_session_id_token_into_daemon_env; env_injection_does_not_affect_daemon_behavior; launches_tools_via_process_launch_no_peer_spawning; gate_admin_register_endpoint_called_with_token; deregisters_session_with_gate_on_daemon_exit; late_hooks_after_deregister_fail_gate_auth_unauthenticated.
server gate-client/engine (9): gate_client_ts_negotiates_subscription_wire_version; streams_three_hookevent_variants; status_mapper_apply_hookevent_no_transcript_io; engine_content_derived_from_hookevent_payload_not_transcript; gate_bridge_feeds_status_and_content_independently; engine_receives_canonical_hookevent_from_gate_subscription; per_session_hookevent_streams_do_not_cross; correlation_id_propagates_through_status_and_content_handlers; proxy_spawn_env_omits_athing_bridge_url.
desktop (3): tauri_uses_process_launch_lib_directly; registers_session_before_bootstrap; deregisters_session_on_daemon_exit.
dep-direction + deploy (9): no_tool_imports_another_tool_internal_modules_cargo_dag; no_tool_depends_on_orchestrator_cargo_dag; only_daemon_pty_client_knows_pty_wire; ts_gate_client_carries_zero_memorya_dependency; gateway_only_deployment_boots_and_serves; full_deployment_boots_and_serves_end_to_end; correlation_id_traces_action_across_daemon_gate_gateway_memory.

## Cross-cutting risks

- Env-var naming consistency across orchestrators / daemon spawn env / notify / gate headers / SetupContext — one canonical set (`ATHING_SESSION_ID`/`ATHING_SESSION_TOKEN`) or hooks silently fail Auth.
- Register-before-spawn race (R4/D7): orchestrator owns ordering; cli installer is passive.
- Deregister authority = daemon PTY session-exit, not a best-effort SessionEnd hook.
- TS/Rust process-launch drift: validate both against ONE table-driven spec (R3/R6/R7).
- v1 fail-open (tool route) loses calls to a restarting backend silently; flips to fail-closed when Firewall lands.
- Correlation id assigned once at gate Router entry; never reassigned (trace join).
- Wire-version drift contracts-rs ↔ sdk; golden cross-check on every shape change.
- service-host adoption side effects in gateway: remove old SIGTERM/manifest code (double-fire / deserialize failures).
- `ATHING_BRIDGE_URL` fallback only valid for legacy standalone; risks masking a misconfigured composed deployment.
- Dep direction (8.4): no tool→tool / tool→orchestrator; only daemon-pty-client knows the PTY wire; TS gate-client zero memorya dep — enforce via Cargo DAG + TS import boundaries.
- Cross-change ordering: 7.7 depends on step-4 transcript retirement (ADR-0017) + gate 5.9; all of step 7 depends on the gate admin surface (5.12); land before daemon-upgrade-drain-restart (8.5).
- Hook-before-subscribe buffering: gate buffers (256 drop-oldest); engine subscribes immediately after spawn.
- macOS Unix-socket path length (~104B): `$ATHING_DIR` + `gate-subscribe.sock`/`gate.sock` must fit.

# Engram / Memory Build Spec (tasks 6.1–6.9)

Synthesized from the `memorya-build-spec` design workflow (6 agents, 2026-06-07), grounded in the real `memorya-rs` + `contracts-rs` code. TDD-first, inline. One commit per `tasks.md` item. Depends on `contracts-rs` (done) and the gate's subscribe wire (5.9 — reconcile gate-client fixtures when the gate build lands).

## Resolved decisions (accepted from design openDecisions)

- **Sync, no async.** memorya stays fully synchronous: `std::thread` + blocking `std::os::unix::net::UnixStream` + a transport-free codec. Do **not** add tokio/async-trait (consistent with the existing zero-async memorya + transport-free `daemon-pty-client`).
- **Concurrency:** `rusqlite::Connection` is `Send` but **`!Sync`** → share `Arc<Mutex<Engram>>` (one serialized connection) across capture + worker + viewer threads. Set `PRAGMA busy_timeout` in `Store::open` as belt-and-suspenders.
- **Capture reuses existing API.** The dispatcher routes `HookKind` → the existing `ensure_session`/`capture_prompt`/`capture_tool` (which already redact via `redact::redact`, skip via `tool_use::should_skip`, and auto-title). Do **not** reimplement a pure `map_hook_event -> chunks` (would double-redact + bypass FK/dedup).
- **Dedicated `capture_queue` table** (literal 6.5) with `{chunk_id, status, attempts, last_error, created_at}` + reclaim-stale (6.6). Ingest commits the chunk first (`insert_chunk -> Option<id>`), then enqueues on `Some(id)` only.
- **Content-hash dedup** (literal 6.5: "dedup by content hash") — add a `content_hash` column + partial UNIQUE index; add `sha2 = "0.10"`. Closes the real gap: `UserPromptSubmit.turn_index = None` prompts are NOT deduped by the existing structural `UNIQUE(session_id,turn_index,kind)` (SQLite treats NULL as distinct).
- **gate-client = transport-free sync codec**, mirrors `daemon-pty-client` framing locally (HEADER_SIZE=4 BE, BODY_SEP=0x0a); `pub const WIRE_VERSION = contracts::HOOK_SUBSCRIPTION_WIRE_VERSION`; imports neither `daemon-pty-client` nor `memorya` (8.4). Transport (UnixStream) lives in memorya's `GateSubscriptionSource`, not in gate-client.
- **Env contract:** composed mode keyed by `ATHING_GATE_URL` + `ATHING_SESSION_ID` (canonical, matching the shipped `athing-notify`; see composition-build-spec). `ATHING_SESSION_TOKEN` carries the per-session token.
- **No service-host in 6.x** — lifecycle/bootstrap is orchestrator scope (7.3–7.4).
- **Provisional gate wire:** the exact hook-event subscribe frame envelope is owned by gate 5.9. gate-client golden fixtures + `decode_subscription_frame` are provisional; **reconcile against the gate's actual `endpoint/subscribe.rs` when the gate build lands** before treating gate-client as final.
- **SessionEnd:** pure skip in v1 (consolidation deferred); revisit later.

## Cargo deps

- NEW `packages/gate-client`: `contracts-rs` (as `contracts`), `serde` (derive), `serde_json`, `thiserror = "2"`. NO tokio, NO daemon-pty-client, NO memorya. Add to root workspace members. package.json (lib turbo), `.gitignore /target`, no per-crate lock/profile.
- `apps/memorya-rs` add: `gate-client = { path = "../../packages/gate-client" }`, `contracts-rs` (as `contracts`), `sha2 = "0.10"`. Already has serde_json/anyhow/rusqlite/redact. NO tokio.

## Crate layout

NEW `packages/gate-client/{Cargo.toml, src/lib.rs, tests/wire_fixtures.rs, package.json, .gitignore}` — `RawFrame`, `FrameDecoder{new,push}`, `encode_frame`, `encode_subscribe_request(&HookSubscribeRequest)`, `enum SubscriptionFrame{HelloAck{version,capabilities}, HookEvent(HookEvent), Other{kind}}`, `decode_subscription_frame(&RawFrame)->Option<SubscriptionFrame>`, `enum DecodeError`, `WIRE_VERSION`.

NEW memorya modules: `src/hook_source.rs` (`trait HookSource: Send { fn next(&mut self)->Option<HookEvent> }`; `StubSource` over `Vec<HookEvent>`; `GateSubscriptionSource` = owned `UnixStream` + `gate_client::FrameDecoder`, blocking reads, hello-ack version check, `Drop` closes socket), `src/capture.rs` (`HookCapturer::dispatch(&HookEvent)` → ensure_session/capture_prompt/capture_tool; PermissionRequest/Stop/SessionEnd skipped; wildcard arm), `src/queue.rs` (`capture_queue` ops: enqueue/drain_batch/mark_embedded/mark_failed/reclaim_stale), `src/worker.rs` (`EmbeddingWorker::spawn(Arc<Mutex<Engram>>, interval, Arc<AtomicBool>)->JoinHandle`; drain loop; `catch_unwind`; reclaim_stale on start), `src/dual_mode.rs` (`enum CaptureMode{Standalone,Composed}` from subcommand + `ATHING_GATE_URL`/`ATHING_SESSION`).

MODIFY memorya: `lib.rs` (mod decls + enqueue helpers), `server.rs` (REMOVE `POST /hook` + `ingest_hook` + 4 hook tests + invalid-json test; KEEP `GET /` viewer + `GET /healthz`; doc "hook ingress"→"viewer only"), `main.rs` (`mcp` = MCP stdio + viewer thread; `serve` = viewer only; resolve CaptureMode; Composed → spawn capture thread + EmbeddingWorker, join on shutdown), `schema.sql` (`capture_queue` table + indexes; `content_hash` column; `PRAGMA busy_timeout`), `store.rs` (busy_timeout in open; content_hash on insert_chunk), `Cargo.toml` (deps). Root `Cargo.toml` (+`packages/gate-client`).

## Build order (one committable unit each; cargo test + clippy --all-targets --locked -D warnings green before commit)

1. **gate-client crate** [6.2] — transport-free sync codec + golden wire fixtures (hello-ack + one HookEvent per HookKind). Provisional until gate 5.9; reconcile then.
2. **HookSource port + capture mapping** [6.3, 6.4] — trait + StubSource + GateSubscriptionSource (unit-tested against an in-process mock unix-socket writing golden frames); HookCapturer routes to existing capture API; ensure_session before any chunk (FK).
3. **Durable capture_queue + proactive worker** [6.5, 6.6] — table + content_hash dedup + busy_timeout; ingest commit-then-enqueue idempotent; EmbeddingWorker thread (drain interval `ATHING_EMBED_DRAIN_INTERVAL_MS` default ~5000, batch ~100), reclaim_stale on start, catch_unwind, survives restart.
4. **Dual-mode startup + remove POST /hook** [6.7, 6.9] — dual_mode detect; main wires faces; delete `POST /hook`+`ingest_hook`+orphaned server tests (behavior re-covered by step-2 capture tests); viewer loopback-only both modes.
5. **Daemon-free + memory-only slice** [6.1, 6.8] — dependency-direction guard tests (memorya lib has no daemon/pty/socket dep; gate-client has no memorya dep; rule 8.4); memory-only slice integration test GATED on gate 5.9 (ignored until live; tracked with 8.2).

## Test inventory (63)

gate_client (12): frame_decoder_holds_partial_frame_across_pushes; frame_decoder_extracts_multiple_frames_in_one_push; frame_decoder_splits_meta_and_body_on_first_0x0a; encode_subscribe_request_carries_session_id_and_wire_version; encode_subscribe_request_round_trips_through_frame_decoder; decode_subscription_frame_parses_hello_ack_version_and_capabilities; decode_subscription_frame_parses_hook_event_for_each_hook_kind; decode_subscription_frame_preserves_correlation_id_unchanged; decode_subscription_frame_returns_other_on_unknown_type; decode_subscription_frame_returns_none_on_invalid_json_meta; wire_version_equals_contracts_hook_subscription_wire_version; subscribe_decode_matches_hook_subscription_wire_fixtures.
hook_source (5): stub_source_yields_events_in_order_then_none; stub_source_next_never_blocks; gate_subscription_source_decodes_events_from_mock_unix_socket; gate_subscription_source_rejects_wire_version_mismatch; gate_subscription_source_closes_socket_on_drop.
capture (12): session_start_ensures_session_row_with_client_and_cwd; user_prompt_submit_captures_prompt_chunk; user_prompt_submit_with_empty_content_creates_no_chunk; post_tool_use_captures_tool_chunk_with_auto_title; post_tool_use_skips_low_value_tool_on_skip_list; permission_request_is_skipped; stop_is_skipped; session_end_is_skipped; ensures_session_before_chunk_insert_satisfying_foreign_key; duplicate_hook_fire_is_idempotent; stub_and_gate_sources_produce_identical_chunks; unknown_hook_kind_is_skipped_without_panic.
queue (7): enqueue_creates_pending_row_for_committed_chunk; enqueue_is_idempotent_for_same_chunk; drain_batch_returns_pending_oldest_first; mark_embedded_removes_row; mark_failed_increments_attempts_and_records_last_error; reclaim_stale_resets_abandoned_rows_on_startup; pending_requests_survive_db_reopen.
lib (3): ingest_commits_chunk_before_enqueuing_embedding_request; embedding_request_enqueued_even_when_worker_not_running; content_hash_dedup_suppresses_duplicate_prompt_without_turn_index.
worker (6): worker_drains_queue_on_configured_interval; worker_embeds_pending_chunks_then_recall_finds_them; worker_stops_on_signal; worker_logs_and_reschedules_on_embed_failure_without_panicking; worker_reclaims_stale_requests_on_startup; background_worker_drains_without_waiting_for_a_recall_request.
integration (1): capture_to_embedding_end_to_end_via_stub_source.
dual_mode (4): mcp_subcommand_selects_standalone_mcp_plus_viewer; serve_subcommand_selects_viewer_only; gate_url_present_selects_gate_subscription_source; gate_url_absent_selects_stub_source.
server (4 keep): post_hook_route_returns_404; get_root_viewer_still_renders_recent_chunks; healthz_still_returns_ok; viewer_binds_loopback_127_0_0_1_only_in_both_modes. (DELETE 5 orphaned: hook_prompt_ingests_chunk, hook_prompt_redacts_secret_before_storage, hook_tool_skips_low_value, hook_tool_stores_with_title, invalid_json_400 — re-covered by capture::.)
deps (3): memorya_lib_has_no_daemon_pty_or_socket_dependency; gate_client_has_no_memorya_or_daemon_pty_client_dependency; memorya_wire_dependencies_are_only_contracts_rs_and_gate_client.
deployment (1, gated on 5.9): memory_only_slice_subscribes_to_gate_without_daemon_or_gateway.

## Cross-cutting risks

- `Connection` `!Sync` → `Arc<Mutex<Engram>>`; set `busy_timeout`.
- FK: `chunks.session_id REFERENCES sessions(id)` (foreign_keys=ON) → `ensure_session` before any chunk insert.
- Capture must route to the side-effectful `capture_prompt`/`capture_tool` (redact+skip+title), not a pure mapper.
- Drop POST /hook removes 4 passing tests — capture behavior must be fully re-covered by step-2 capture tests (no net coverage loss).
- gate hook-event frame envelope owned by gate 5.9 (unbuilt at design time) — gate-client fixtures provisional; reconcile when the gate lands; live memory-only integration gated until then.
- `correlation_id` round-trips unchanged gate → HookEvent → logs; codec/capture must not mint or drop it.
- `StubEmbedder` is `#[cfg(test)] pub(crate)` — keep worker/queue tests in-crate or expose a deterministic test embedder.
- Dep direction (8.4): gate-client reimplements framing locally, imports neither daemon-pty-client nor memorya; memorya imports gate-client, never the reverse — guard with a cargo-metadata test.
- Semantics flip: `memorya serve` becomes viewer-only; `memorya mcp` additionally starts the viewer — document + test both.

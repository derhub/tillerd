## 0. Decision records (ADR)

- [x] 0.1 Author ADR-0016 (daemon becomes pty-only; hook ingress and fan-out move to the gate; records the composable-tools topology) — amends the hook-ingress clause of ADR-0008.
- [x] 0.2 Author ADR-0017 (structured content from the canonical `HookEvent` payload; `parseTranscriptEntry`/`transcriptPath` removed; adapter is single-language Rust `parse_hook`) — supersedes ADR-0006.
- [x] 0.3 Flip ADR-0006 status to `superseded by ADR-0017`; annotate ADR-0008 status as `hook-ingress clause amended by ADR-0016`.

## 1. Contracts and shared libraries (foundation)

- [x] 1.1 Create `contracts-rs` (Rust): the shared **wire types** — `HookEvent` + `HookKind` (freeze design D11 / decision R1 verbatim), `SessionId`, `CorrelationId`, and the subscription + tool-route message shapes; serialize camelCase; expose independent `WIRE_VERSION` consts for the daemon session-event wire and the gate hook-subscription wire (R9); `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`. (The middleware framework is NOT here — it lives in the gate binary, task 5.1.)
- [x] 1.2 Mirror `HookEvent`/`HookKind` + correlation id in `@athing/sdk` (TypeScript); add a golden cross-check that the two encodings match.

## 2. service-host (run-me) and process-launch (run-others)

- [x] 2.1 `service-host`: deterministic dir/manifest/socket path resolution honoring the base-dir override.
- [x] 2.2 `service-host`: atomic manifest write (temp + rename) carrying `{pid, version}`; remove on clean stop.
- [x] 2.3 `service-host`: signal-driven graceful shutdown (escalating, no orphans) and an unauthenticated liveness probe.
- [x] 2.4 `service-host`: `host::run(service)` entry that performs 2.1–2.3 and invokes the tool's serve behavior; unit-test paths/manifest/shutdown.
- [x] 2.5 `process-launch`: adopt-or-spawn (connect to live instance only on **exact** manifest-`version` string match per R3, else spawn and wait until reachable; overwrite stale manifest). Resolve `ATHING_DIR` with parity to the existing TS/Rust behavior (R7).
- [x] 2.6 `process-launch`: `spawn_fields_differ` comparing only the spawn-affecting set `{command, args, cwd, env[allowlist]}` (R6; per-tool declared env-key allowlist), ignoring logging level / observer / manifest-only fields; + bounded-backoff restart; unit-test both.

## 3. PTY daemon becomes pty-only

- [x] 3.1 Remove hook ingress from the daemon (`hooks.sock`, `relay_hook`, the negotiated hook capability) per the `rust-pty-daemon` REMOVED requirement.
- [x] 3.2 Expose the consumer-oblivious session-event subscription (bytes + lifecycle by session id) as the daemon's public surface; version the wire.
- [x] 3.3 Migrate the daemon onto `service-host` (manifest/sockets/signals/liveness).
- [x] 3.4 Publish the Rust `daemon-pty-client` (PTY wire only); align the TS engine's `proxy.ts` with the versioned spec (not a rewrite) — remove the proxy's hook-frame branch (the `adapter.parseHook` call on daemon frames) now that the daemon relays no hooks. (Engine TS verification deferred to step 4 rework; proxy edit unverifiable while engine is mid-migration red.)
- [x] 3.5 Verify a daemon-only deployment serves a plain terminal with no hook ingress. (Covered by `advertised_capabilities_carry_no_hook_face` + daemon serve tests + standalone `host::run`; live PTY-only deployment-slice e2e tracked under 8.2.)

## 4. AgentAdapter (parse_hook) + retire TS hook/transcript parsing

- [x] 4.1 Define the `AgentAdapter` trait (`parse_hook`); implement it as a v1 module in the gate binary (the v1 agent) producing the canonical `HookEvent`; unit-test one case per event type (raw → expected event). No separate adapter crate yet — promote to a per-agent adapter crate only at multi-agent (ADR-0003).
- [x] 4.2 Retire the TS `parseHook` for hooks (engine consumes canonical events from the gate).
- [x] 4.3 Remove `parseTranscriptEntry`/`transcriptPath` and the engine's transcript reader (ADR-0006 superseded by ADR-0017); engine content now comes from the `HookEvent` payload.
- [x] 4.4 Update the SDK adapter contract: drop `parseHook`/`parseTranscriptEntry`/`transcriptPath` from `AgentDefinition` (`@athing/sdk`); the TS agent adapter package carries no parse functions (delete it if nothing remains, else reduce it to declarative config only). Fix all references and tests.

## 5. The gate

- [x] 5.1 Gate framework modules (in the gate binary): `Kind`, `Ctx`, `Outbound`, `Reject` (thiserror), `Flow = Result<Outbound, Reject>`, `Next`, the `Middleware` trait, `seq`/`par` combinators, and `Next::noop()`/`Next::spy()` test helpers; unit-test `seq` (short-circuit on first `Err`) and `par` (joined). No `gate-core` crate.
- [x] 5.2 `Auth` middleware: constant-time per-session token compare against the in-memory session registry `sessionId -> {tokenHash, allowPolicy}` (R4; no on-disk policy file in v1); unit-test pass/reject.
- [x] 5.3 `Observe` middleware (around): bind `{sessionId, correlationId, component:"gate"}` (ADR-0012), construct with a Resource identity, emit a record `{ts, kind, eventType?, outcome, latencyMs, fanoutN?, droppedN?}`; unit-test with a fake sink.
- [x] 5.4 `Normalize` middleware: call the injected adapter, set `ctx.event`, reject on parse error; unit-test.
- [x] 5.5 `FanOut` middleware (terminal): publish the canonical event to per-session subscribers via `par`; unit-test fan-out to N.
- [x] 5.6 `PassThrough` middleware (terminal) for the tool route.
- [x] 5.7 Router: dispatch by `Kind`, globals `[Observe, Auth]` (Observe outermost), Hook + Tool routes; `gate.handle(inbound) -> Flow`.
- [x] 5.8 Hook endpoint (loopback HTTP, per-session token) reachable by a simple posted command; transport-level max-body-size cap (OOM guard).
- [x] 5.9 Hook-event subscription wire (own `WIRE_VERSION` per R9, mirrored in contracts + sdk); bounded per-session delivery capacity 256, drop-oldest, increment `droppedN` + log once per lag, override via `ATHING_GATE_QUEUE_CAP` (R2).
- [x] 5.10 Correlation id: accept caller-supplied, assign when absent, preserve through middleware → emitted event → logs.
- [x] 5.11 Tool route entry over local IPC (length-prefixed loopback framing) for `ToolCall`/`ToolResult`.
- [x] 5.12 Face isolation: hook endpoint ≠ tool route ≠ admin; admin on a separate authenticated surface; health endpoint unauthenticated (liveness/version).
- [x] 5.13 Migrate the gate onto `service-host`.
- [x] 5.14 Router integration tests: `handle(Hook)` fans out to N; `handle(ToolCall)` returns `Forward`; auth reject; face isolation (tool caller cannot publish a hook); subscription teardown; correlation preserved.

## 6. Memory: engram-rs library + thin tool

- [x] 6.1 Ensure `engram-rs`'s existing library is daemon-free (storage, recall, embeddings, chunking, consolidation); the memory tool is the thin bin over it. No new `engram-core` crate.
- [x] 6.2 Create `gate-client` (Rust lib): `subscribe(session) -> HookEvent` stream decoding the gate hook-subscription wire (task 5.9) from `contracts-rs`; unit-test decode against wire fixtures. Thin, focused, no shared client crate.
- [x] 6.3 `HookSource` port with a gate-subscription adapter over `gate-client` (memory subscribes to the gate); add a stub source for capture unit tests.
- [x] 6.4 Capture consumes the canonical `HookEvent` and maps it to chunks (no raw-format parsing).
- [x] 6.5 Durable capture-queue table; ingest committed first then an embedding request enqueued; make ingest idempotent (dedup by content hash).
- [x] 6.6 Convert the lazy `embed_pending` flush to a proactive background worker (embedding on a blocking pool); queue survives restart; reclaim stale on startup.
- [x] 6.7 Recall dual-mode (R5): face by subcommand (`engram mcp` = standalone MCP stdio + viewer; `engram serve` = viewer only); capture source by env (`ATHING_GATE_URL` present -> subscribe to the gate when composed; absent -> standalone stub/none); retain the loopback human viewer in both modes.
- [x] 6.8 Verify a memory-only deployment = memory + gate (memory subscribes; no daemon, no MCP gateway).
- [x] 6.9 Remove memory's own hook ingress: delete the `POST /hook` route + `ingest_hook` from `engram-rs`'s loopback server (capture now arrives via the `HookSource` gate subscription); keep only the `GET /` human viewer. This retires the second of the two hook ingresses the change consolidates.

## 7. Tool gateway + orchestrators (thin)

- [x] 7.1 Tool gateway: adopt `service-host`; send `ToolCall`/`ToolResult` inbounds to `gate.handle()` (observe pass-through); **fail-open** if the gate is unreachable (log + proceed).
- [x] 7.2 Tool gateway: register memory as an ordinary backend when composed (no special-casing); the gateway spawns only external backends.
- [x] 7.3 Orchestrators (server, desktop, cli): bootstrap chosen tools via `process-launch`; remove all peer-spawning from tools.
- [x] 7.4 Orchestrator session lifecycle: mint `{session, token}`, register with the gate admin surface **before** spawn (R4), inject `{ATHING_GATE_URL, ATHING_SESSION_ID, ATHING_SESSION_TOKEN}` into the daemon spawn env (canonical names per composition-build-spec; daemon passes through, stays oblivious); composed memory reads the same `ATHING_GATE_URL` to subscribe (R5).
- [x] 7.5 Orchestrator teardown: observe the daemon session-exit and deregister the session from the gate; late hooks then fail auth.
- [x] 7.6 cli hook installer: target the gate (the universal ingress); memory subscribes for events.
- [x] 7.7 Server: host the gate hook-subscription client (feeds the engine) alongside `daemon-pty-client`; engine maps `HookEvent.type → status` and `payload → content`.

## 8. Composition verification

- [x] 8.1 Verify each `specs/*` scenario is covered by a test (gate, daemon-session-subscription, service-host, process-launch, tool-composition, engram-capture, engram-recall, rust-pty-daemon).
- [x] 8.2 Verify the four deployment slices run: memory-only, gateway-only, PTY-only, and full (daemon + gate + tool gateway, memory as library).
- [x] 8.3 Verify observability: every inbound emits a correlation-bound record; one `correlationId` traces an action across daemon → gate → gateway → memory.
- [x] 8.4 Confirm dependency directions: no tool → tool, no tool → orchestrator; only `daemon-pty-client` knows the PTY wire; the daemon depends on nothing downstream.
- [x] 8.5 Coordinate ordering with the `daemon-upgrade-drain-restart` change (land this change first; the daemon spec deltas touch different requirements).

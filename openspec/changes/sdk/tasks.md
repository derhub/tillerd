## 1. Workspace scaffolding

- [ ] 1.1 Create `packages/sdk`, `packages/engine`, `packages/adapter-claude-code`, `apps/server`, `apps/ui` in the turbo workspace
- [ ] 1.2 Wire turbo pipelines (build, dev, lint, check-types, test) and TS project refs with the inward dep direction (engine→sdk, adapter→sdk, apps→engine+adapter+sdk, ui→sdk types)
- [ ] 1.3 Add pinned dependencies: `node-pty@1.1.0` and `valibot` to `@athing/engine`; React + react-router + `@xterm/xterm` + fit addon to `apps/ui`
- [ ] 1.4 Add a CI check that `node-pty` builds/loads under Bun

## 2. SDK contracts (`@athing/sdk`) — zero deps, zero impl

- [ ] 2.1 Define the canonical event model: data (bytes), status enum `{ IDLE | WORKING | WAITING_INPUT | DONE }`, content event types (tool_use, edit, usage)
- [ ] 2.2 Define the `HookEvent` contract `{ sessionId, type, payload? }` with the type enum (SessionStart, UserPromptSubmit, PostToolUse, PermissionRequest, Stop, SessionEnd)
- [ ] 2.3 Define the typed error taxonomy (`BinaryNotFound`, `NotAuthenticated`, `SpawnFailed`, `HookInstallFailed`, `TranscriptUnavailable`, `TransportClosed`, `Timeout`, `VersionUnsupported`)
- [ ] 2.4 Define the `AgentDefinition` contract: config data (launch template, hook-install spec, CLI version range) + function signatures (`parseHook`, `transcriptPath`, `parseTranscriptEntry`)
- [ ] 2.5 Define the `AgentSession` contract (`send`, `input`, `interrupt`, `resize`, `kill`) + the `Engine` factory/start/resume/shutdown surface + session options
- [ ] 2.6 Export valibot schemas for `HookEvent`, content events, and adapter config data

## 3. Engine — PTY drive plane

- [ ] 3.1 Binary resolution: `CLAUDE_CODE_EXECUTABLE` → login-shell PATH → common locations; fail with `BinaryNotFound`
- [ ] 3.2 Clean launch: spawn `$SHELL -lc 'exec <binary> <args>'` in a PTY so only the agent TUI reaches the stream (no shell prompt/echo)
- [ ] 3.3 Raw byte I/O: stream PTY output unmodified (no ANSI strip, no re-decode); write raw input bytes
- [ ] 3.4 Add a no-encoding-hops test (ANSI + multibyte UTF-8 pass through byte-identical)
- [ ] 3.5 `resize(cols, rows)` propagates to the PTY
- [ ] 3.6 Prompt submission: bracketed-paste the text + submit key; raw `input(bytes)` passthrough; `interrupt()` writes the interrupt key
- [ ] 3.7 Teardown: SIGTERM → grace period → SIGKILL; capture exit code/signal; free the PTY; emit exit event
- [ ] 3.8 First-run handling: launch flags to skip trust/onboarding; detect not-logged-in → `NotAuthenticated` within startup timeout

## 4. Engine — hook ingress + HookEvent seam

- [ ] 4.1 Loopback receiver on an ephemeral `127.0.0.1` port; handle port-in-use
- [ ] 4.2 Install hooks once into the agent's settings (non-destructive merge) + an explicit uninstall path
- [ ] 4.3 Inject bridge URL + session id + per-session token into each session's PTY env at launch
- [ ] 4.4 Verify the per-session token on every callback; reject missing/mismatched
- [ ] 4.5 Validate the envelope, call `adapter.parseHook` → `HookEvent`, route by session id; drop unknown-session callbacks
- [ ] 4.6 Idempotent dispatch (duplicate callbacks do not corrupt state)
- [ ] 4.7 Expose a `dispatchHook(HookEvent)` path so any producer (incl. tests) drives the engine identically

## 5. Engine — status + content planes

- [ ] 5.1 Status mapper: generic contract-enum → 5-state mapping; idempotent; emit transitions
- [ ] 5.2 Transcript reader: read-on-hook (PostToolUse/Stop) + final read on exit; no watcher/poll
- [ ] 5.3 Byte-offset delta tracking; reset + re-read on truncation/identity change
- [ ] 5.4 Emit typed content via `adapter.parseTranscriptEntry`; treat missing transcript as empty (`TranscriptUnavailable`)

## 6. Engine — session core + reliability

- [ ] 6.1 `createEngine()` factory: isolated instance, session registry, no global state; instance-scoped `shutdown()`
- [ ] 6.2 `AgentSession` orchestration: wire drive + ingress + status + content; fan out onData/onStatus/onContent
- [ ] 6.3 Ready-gating: queue `send` until IDLE (bounded queue; overflow → typed error); first prompt waits for ready
- [ ] 6.4 Timeouts (startup, shutdown grace, idle) → typed errors + defined transitions
- [ ] 6.5 Backpressure: bounded per-session buffer with PTY pause/resume (or logged drop policy)
- [ ] 6.6 Independent plane degradation: status/content failure reported as typed error, session survives
- [ ] 6.7 CLI version detection vs adapter range → `VersionUnsupported`
- [ ] 6.8 Resume: `start({ resume: sessionId })` relaunches against the prior id
- [ ] 6.9 Session-correlated structured logs; opt-in raw-I/O capture (off by default, redacted)

## 7. Claude Code adapter (`@athing/adapter-claude-code`)

- [ ] 7.1 Launch config: `claude` with caller-chosen `--session-id` + `--dangerously-skip-permissions`, no API key
- [ ] 7.2 Hook-install spec targeting `~/.claude/settings.json` (SessionStart, UserPromptSubmit, PostToolUse, PermissionRequest, Stop, SessionEnd) with a portable notify command (no `curl` assumption)
- [ ] 7.3 `parseHook`: Claude payload → `HookEvent` (type + sessionId) + unit tests with golden fixtures
- [ ] 7.4 `transcriptPath`: `~/.claude/projects/<encoded-cwd>/<id>.jsonl` incl. cwd-encoding rule + tests
- [ ] 7.5 `parseTranscriptEntry`: tool_use/edit/usage extraction + golden-fixture tests
- [ ] 7.6 Declare the supported `claude` CLI version range

## 8. Integration

- [ ] 8.1 End-to-end test against a real `claude`: start → ready → send → observe onData bytes, onStatus transitions, onContent events → interrupt → kill
- [ ] 8.2 Concurrent-sessions test: two sessions in one engine instance, isolated
- [ ] 8.3 Failure-path tests: BinaryNotFound, NotAuthenticated, startup timeout, hook-install failure, transcript-unavailable degradation

## 9. Dev harness apps

- [ ] 9.1 `apps/server`: composition root — create engine, inject `claudeCode` adapter; expose a session over WebSocket + HTTP
- [ ] 9.2 Validated wire protocol (valibot) for client↔server messages; reject malformed
- [ ] 9.3 `apps/ui`: react-router SPA — xterm terminal from the byte stream; send keystrokes/resize/interrupt/prompt back
- [ ] 9.4 `apps/ui`: status indicator + content panels (tool calls, usage)
- [ ] 9.5 Manual verification: drive a real Claude Code session end-to-end through the web UI

## 10. Docs

- [ ] 10.1 `@athing/sdk` README: public API + how to write an adapter
- [ ] 10.2 Document the auth/billing/ToS constraint (one subscription = one user; API-key/Commercial for multi-user) and the hook uninstall path

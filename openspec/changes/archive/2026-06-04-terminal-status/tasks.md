## 1. Wire contract (sdk)

- [x] 1.1 Reuse the existing `SessionStatus` type (`packages/sdk/src/types/events.ts`) — do NOT add a new enum or status value.
- [x] 1.2 Add a `StatusFrame` (`{ type: "status"; sessionId; status; source: "hook" | "terminal" }`) to the typed server-frame set and the frame parser/validation (`packages/sdk/src/protocol/messages.ts`), so clients can parse it. Terminal-`source` status is limited to `IDLE` | `WORKING`.
- [x] 1.3 Export the new symbols from `packages/sdk/src/index.ts`.
- [x] 1.4 Note in code/docs that combining agent status and terminal status into one displayed value is a consumer/presentation concern, out of scope here.

## 2. Terminal status derivation (daemon-pty)

- [x] 2.1 Track `last_output_at` per session, updated in the existing reader loop on every output chunk.
- [x] 2.2 Sample the pseudo-terminal foreground process group (`MasterPty::process_group_leader()`) and compare to the session root `process_id`; when it returns `None`, degrade to output-quiescence alone.
- [x] 2.3 Compute terminal status: `WORKING` when a job other than the root holds the foreground group, or output is within the quiescence threshold; `IDLE` when the root holds the foreground group and output has been quiet past the threshold. Emit only `IDLE` or `WORKING` — never `WAITING_INPUT`.
- [x] 2.4 Do NOT emit a completion status — completion stays on the existing `exit` frame; stop sampling once the session has exited.
- [x] 2.5 Establish an initial terminal status on spawn; drive sampling from a periodic tick (~250ms) and emit a status frame only on transition.
- [x] 2.6 Make the quiescence threshold a single named constant (~300-500ms).
- [x] 2.7 Ensure derivation works for adopted sessions (inherited master fd, no reaper): foreground group via the adopted fd, output activity from the reader.

## 3. Status frame wiring + engine integration

- [x] 3.1 Construct the `status` frame in daemon-pty and push it through the existing subscriber broadcast; deliver the current terminal status to a client on subscribe (do not wait for the next transition).
- [~] 3.2 DEFERRED (future work): the reference daemon (`packages/daemon`) runs on a runtime with no native `tcgetpgrp`, so it omits emission rather than diverge. `StatusFrame` stays defined in the shared contract; only daemon-pty emits for now.
- [x] 3.3 Contract-conformance fixture: daemon-pty's emitted `status` frame parses to the shared `StatusFrame` contract with matching field names/values (golden JSON asserted on both the Rust and the sdk side).
- [x] 3.4 Surface terminal status in the engine as a signal distinct from the hook-derived agent status, tagged by `source`. Do NOT route it through the hook-only `StatusMapper.apply` path (`packages/engine/src/session/status.ts`); confirm existing idle-timeout / prompt-queue logic in `packages/engine/src/daemon/proxy.ts` continues to consume only the agent status.

## 4. Verify and wrap up

- [x] 4.1 Daemon test: shell session running a sub-job reports `WORKING`; at an idle prompt reports `IDLE`.
- [x] 4.2 Daemon test: a single long-lived foreground program toggles `WORKING`/`IDLE` purely on the quiescence threshold and never reports `WAITING_INPUT`.
- [x] 4.3 Daemon test: `process_group_leader() == None` degrades to quiescence without failing; an adopted session derives status without a reaper.
- [x] 4.4 Engine test: a `source: "terminal"` frame does not drive idle-timeout / prompt-queue logic.
- [x] 4.5 Confirm no new dependencies added to `daemon-pty` or `sdk`; `sdk` stays Web-API-only.
- [x] 4.6 Validate acceptance criteria from the delta spec; prepare concise reviewer notes.

# 0006. Structured content from transcript read-on-hook

- Status: accepted
- Date: 2026-06-01

## Context

PTY output is opaque bytes, so structured content (tool calls, edits, usage/cost) must come from elsewhere. The agent writes a session transcript (JSONL) to disk. We need a way to surface that content without a file watcher (flaky cross-platform) or a polling timer, and without coupling content to a particular transport.

## Decision

Recover structured content by reading the on-disk session transcript, triggered by the hook plane: on PostToolUse and Stop the engine reads the transcript delta and calls the adapter's `parseTranscriptEntry` to emit typed content; a final read happens on process exit. The reader tracks a byte offset for incremental reads and resets if the file shrinks below the offset or its identity changes (rewrite/compaction). The emitted content shape is transport-independent — the same shape a future stream-json mode would emit.

## Consequences

- No file watcher and no polling timer; content is tied to events the engine already receives.
- Content granularity is per-tool/per-turn, not sub-tool streaming — acceptable because the PTY already shows live activity.
- Content shares the hook trigger with status, so a dead hook plane reduces content to the exit-time read; a missing transcript is treated as empty (`TranscriptUnavailable`), not a crash.
- One canonical content model across current and future transports.

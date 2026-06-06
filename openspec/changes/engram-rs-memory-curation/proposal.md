## Why

The `engram-rs-memory-layer` change ships capture, recall, consolidation, and archive but deliberately leaves out the differentiating layer: a compact global memory that carries user preferences, feedback, and a cross-project index into every session, plus a populated temporal fact graph. That layer needs one model call per day, and the model to call is undecided given the host's bring-your-own-login premise (no API key). This change adds curation once that decision is made.

## What Changes

- Add a once-daily curation job that reads the day's digest and the current global memory file and writes an updated, size-bounded global memory file.
- Populate the temporal fact graph (schema already created by the foundation change) by extracting facts from the digest, applying temporal supersession.
- Inject the global memory file into every session at start.
- Add a one-shot bootstrap that runs curation over historical digests in chronological order.
- Resolve the curation-model decision (drive the agent over the existing PTY path, a local model, or an opt-in key) before implementation.
- **BREAKING**: none — additive on top of the foundation change.

## Capabilities

### New Capabilities

- `engram-curation`: the once-daily model call that updates the global memory file and populates the temporal fact graph, plus one-shot bootstrap over historical digests.

### Modified Capabilities

<!-- none — additive on the foundation change -->

## Impact

- Depends on `engram-rs-memory-layer` (storage, capture, consolidation) being in place.
- Introduces the first model dependency in engram and the global memory file under `~/.athing/memories/MEMORY.md`.
- Pending decision: which model the daily job calls under bring-your-own-login, and the size bound / recent-digest window for the global memory file.

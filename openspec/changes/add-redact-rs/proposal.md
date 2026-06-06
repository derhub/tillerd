# Sensitive-data redaction tool

## Why

The memory layer persists captured prompts and tool responses verbatim into a durable,
searchable store (SQLite + embeddings). Any secret or personally identifiable value that
appears in a prompt or tool output — an access key from a file read, an environment dump
from a shell command, an email or card number pasted into a session — is therefore stored
and embedded in plaintext, where it survives recall, consolidation, and digests. There is
no automatic protection today. A reusable redaction tool is needed, applied at the capture
chokepoint before any write, and available to other sinks (such as the MCP gateway) later.

## What Changes

- Add a new standalone Rust crate `packages/redact-rs`: a pure, deterministic redaction
  library plus a thin `stdin -> stdout` CLI, usable by any consumer.
- The library follows the industry-standard detect-then-transform pipeline:
  - A regex catalog of well-known credential shapes (cloud access-key identifiers,
    version-control personal access tokens, provider secret keys, JSON Web Tokens,
    PEM private-key blocks) and structured PII (email, phone, government identifier,
    payment card validated by checksum, IP and hardware addresses), vendored from a
    Presidio-compatible pattern set.
  - A Shannon-entropy fallback for unknown secret formats, gated by length and character
    class to bound false positives.
  - An allowlist/stopword layer (version-control object hashes, UUIDs, version-number-like
    sequences) to suppress structural false positives.
- Transform: detected content is replaced with a single fixed marker `[REDACTED]` (no
  detected-class label, no hash). For a labeled key/value (`KEY=value`, `"key": "value"`,
  authorization header, query parameter) only the value is replaced; the key/label is kept.
  A bare value is replaced whole. Fail closed when a match cannot be cleanly bounded.
- Wire `@athing/engram` capture to the crate: `capture_prompt` and `capture_tool` redact
  content (and the tool-input-derived title) before `ingest`. Documents stay verbatim.

## Capabilities

### New Capabilities

- `redaction`: the redaction library + CLI contract — detection (pattern + entropy +
  allowlist) and transform (`[REDACTED]`, value-only for labeled pairs, fail-closed).

### Modified Capabilities

- `engram-capture`: re-introduce capture-time redaction — prompts and tool events (response
  body and input-derived title) are redacted via the `redaction` library before any write;
  project documents remain indexed verbatim.

## Impact

- Code: new `packages/redact-rs` (`Cargo.toml`, `src/lib.rs`, `src/patterns.rs`,
  `src/entropy.rs`, `src/main.rs`); workspace membership; `apps/engram-rs/Cargo.toml`
  (depend on `redact-rs`) and `apps/engram-rs/src/lib.rs` (call redaction in
  `capture_prompt`/`capture_tool`).
- Dependencies: `regex` (pattern catalog) in `redact-rs`; entropy/allowlist/Luhn hand-rolled.
- Behavior: stored chunk content gains `[REDACTED]` markers; recall/consolidation consume
  redacted text unchanged. False-positive tuning affects recall quality.
- Out of scope: unstructured PII requiring NER (person names, addresses); the daemon
  raw-byte planes; retroactive redaction of already-stored chunks; wiring the MCP gateway
  (separate later change).

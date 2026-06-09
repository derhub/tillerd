## Context

`@athing/memorya` persists captured prompts and tool events as chunks (SQLite + embeddings)
via a single `ingest` write path. Redaction was previously a manual `<private>`-span strip
and has been removed; nothing protects the store today. This change introduces a reusable
redaction crate and re-wires capture to it. Rust workspace already hosts library crates under
`packages/` (`daemon-pty`, `platform-rs`); `regex` is the de-facto pure-Rust engine and is
compatible with the bundled, offline build.

## Goals / Non-Goals

**Goals**

- A pure, deterministic redaction library reusable across sinks, plus a stdin→stdout CLI.
- Detect credentials + structured PII by pattern, unknown secrets by entropy; suppress
  structural false positives by allowlist.
- Replace with a fixed `[REDACTED]` marker; value-only for labeled pairs; fail closed.
- Wire memorya capture (prompt, tool body, tool-input title) before `ingest`.

**Non-Goals**

- Unstructured PII / NER (names, addresses) — no model introduced.
- Daemon raw-byte planes; retroactive redaction; MCP-gateway wiring (later change).

## Decisions

### D1: Standalone crate `packages/redact-rs`

A workspace library crate (`memorya` depends on it), plus a `redact` binary (`src/main.rs`)
that pipes stdin→stdout. Public API: `redact(input: &str) -> String`. Pure, no I/O in the lib.
Rationale: redaction is a cross-cutting concern; a shared crate lets the MCP gateway adopt the
same logic later without duplication. A library (not a service) avoids IPC/latency.

### D2: Detection pipeline (order)

1. Scan with the pattern catalog (D3) and entropy heuristic (D4), collecting candidate spans.
2. Drop candidates matching the allowlist (D5).
3. For low-confidence numeric classes, apply the extra gate (Luhn for cards; context keyword
   for phone) before keeping the candidate.
4. Merge overlapping spans, then apply the transform (D6) left to right.

### D3: Pattern catalog, compiled once

A fixed `&[(Class, Regex)]` compiled behind `OnceLock`, vendored from a Presidio-compatible
set. Credentials: GitHub (`(ghp|gho|ghs|ghu|github_pat)_…`), AWS (`AKIA[0-9A-Z]{16}`),
provider (`sk-(ant|proj)?-…`), JWT (`eyJ….….…`), Slack (`xox[bpoas]-…`), GitLab (`glpat-…`),
PEM private-key blocks. Structured PII: email, IP v4/v6, MAC, US-SSN, IBAN, credit card,
phone. Patterns anchored and bounded; `regex` guarantees linear-time matching.

### D4: Entropy fallback, gated

Tokenize on whitespace/delimiters; flag a token only when it clears all gates — minimum
length, restricted charset (hex/base64-like), and a Shannon-entropy threshold (bits/char).
Single linear pass. Thresholds are constants, tuned against the memorya eval corpus.

### D5: Allowlist / stopwords

Suppress entropy candidates matching: version-control object hashes (hex len 7/40/64),
UUIDs, version-number-like sequences. Pattern-catalog hits are high-confidence and not
subject to allowlist suppression (except the low-confidence numeric gate in D2.3).

### D6: Transform — fixed `[REDACTED]`, value-only for labeled pairs

Replace each span with `[REDACTED]`. Labeled key/value patterns (`KEY=value`,
`"key": "value"`, `Authorization: …`, `?key=value`) capture the value as a submatch;
replace only that submatch, leaving the key/label/separator intact. Bare matches replace the
whole span. No class label, no hash (deferred — avoids a correlation/inference surface).

### D7: memorya wiring

`apps/memorya-rs/Cargo.toml` depends on `redact-rs`. In `lib.rs`, `capture_prompt` wraps
`content` with `redact::redact`; `capture_tool` redacts both the `auto_title` output and the
response body before composing `content`/`title`. `ingest` and document indexing unchanged.
Digests/facts/entities derive from stored (redacted) chunks — no raw-input path.

## Risks / Trade-offs

- False positives degrade recall → pattern-first; entropy gated by length+charset+threshold;
  allowlist; thresholds tuned on the eval corpus; fixtures cover boundaries.
- Structured-PII FP (version string as IP, 16-digit id as card, numeric run as phone) →
  anchor patterns, require Luhn for cards, context-gate phone, allowlist version-like runs.
- New `regex` dependency → accepted; standard, pure-Rust, linear-time (no catastrophic
  backtracking).
- Pre-existing stored chunks remain unredacted → out of scope (forward-only); pre-v1, no shim.

## Migration Plan

Forward-only; no schema change. Newly captured chunks gain `[REDACTED]` markers. No backfill
(prune + re-index for a clean store). Rollback: drop the memorya dependency call; stored data
remains readable (markers are plain text).

## Open Questions

- Exact entropy length/threshold constants — resolve on the eval corpus before locking.
- Externalize the pattern catalog / allowlist as config later? Deferred; built-in for now.
- Adopt per-class confidence scoring (Presidio-style) vs binary high/low gating? Start binary.

## 1. Crate scaffold

- [x] 1.1 Create `packages/redact-rs/Cargo.toml` (lib `redact` + bin `redact`; `regex` dep) and add to workspace members
- [x] 1.2 Create module skeleton: `src/lib.rs`, `src/patterns.rs`, `src/entropy.rs`, `src/main.rs`

## 2. Detection

- [x] 2.1 `patterns.rs`: `OnceLock` catalog of `(class, Regex)` for credentials + structured PII, vendored from the Presidio-compatible set
- [x] 2.2 `patterns.rs`: Luhn check for payment cards; context-keyword gate for phone
- [x] 2.3 `entropy.rs`: Shannon entropy + length/charset gates for unknown secrets
- [x] 2.4 `lib.rs`: allowlist (SHA/UUID/version-like) suppression
- [x] 2.5 `lib.rs`: collect candidate spans, apply gates, merge overlaps

## 3. Transform

- [x] 3.1 `lib.rs`: replace spans with `[REDACTED]`; value-submatch only for labeled key/value pairs; bare match replaced whole; fail-closed
- [x] 3.2 `lib.rs`: public `redact(input: &str) -> String`; clean input returned unchanged

## 4. CLI

- [x] 4.1 `main.rs`: read stdin, `redact`, write stdout, exit zero

## 5. Tests

- [x] 5.1 Unit tests: each credential/PII class hit, entropy boundary, allowlist suppression, Luhn gate, phone context gate
- [x] 5.2 Unit tests: `[REDACTED]` output, labeled-pair key preserved, clean input unchanged, fail-closed
- [x] 5.3 CLI test: pipe secret through stdin → `[REDACTED]` on stdout, exit zero

## 6. Wire memorya capture

- [x] 6.1 `apps/memorya-rs/Cargo.toml`: depend on `redact-rs`
- [x] 6.2 `lib.rs` `capture_prompt`: redact `content` before `ingest`
- [x] 6.3 `lib.rs` `capture_tool`: redact `auto_title` output and response body before composing
- [x] 6.4 memorya test: prompt + tool capture store `[REDACTED]`; document indexing stays verbatim

## 7. Verify

- [x] 7.1 `cargo clippy --all-targets -- -D warnings` clean; `cargo test` green for both crates
- [x] 7.2 Runtime: drive `memorya serve` → `POST /hook` with a secret → stored chunk shows `[REDACTED]`

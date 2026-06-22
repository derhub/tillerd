## Context

tillerd is a polyglot monorepo (Rust `crates/`, TS `apps/` + `packages/`, Cargo workspace + Turborepo). CI runs bun lint/check-types/test, the Rust build, and e2e. No structural-rule enforcement exists today — conventions live in docs, memory, and reviewers' heads and regress silently. clippy and the TS linter reason within a file or crate; they cannot express a rule like "a struct that implements `Command<Ctx>` must carry only plain built-in fields". ast-grep already backs structural code search in this project, so adopting it for enforcement keeps to one tool. Verified locally against the real tree with ast-grep 0.43.0.

## Goals / Non-Goals

**Goals:**

- One `sgconfig.yml` + `.ast-grep/` tree that a single `ast-grep scan` evaluates, plus `ast-grep test` for rule fixtures.
- Rule files organized by the layer they govern; per-rule `files:` scoping.
- A blocking CI gate that fails on `error`-severity findings and a shared-utility mechanism for reuse.
- Seed exactly one rule, `command-query-dto`, shipping as `warning` until the DTOs are migrated.

**Non-Goals:**

- No second rule yet (import boundaries, handler hygiene, sqlx macro ban, module visibility are deferred re-seeds).
- No codemod/autofix (`fix:`) rules; structural enforcement only.
- No DTO refactor here — flipping `command-query-dto` to `error` is follow-up work.
- No cross-repo rule-pack sync; reuse is recipe-level (global `tools/ast-grep.md`).

## Decisions

**D1 — ast-grep loaded via `sgconfig.yml`; no command-line wiring.** `ruleDirs: [.ast-grep/rules]`, `testConfigs: [{testDir: .ast-grep/tests}]`, `utilDirs: [.ast-grep/utils]`. `ast-grep scan` and `ast-grep test` read the config and need no `-f` or path argument. *Alternative rejected:* passing `-f <dir>` per invocation — more fragile, and `ast-grep test` needs `testConfigs` in the config regardless. *Gotcha:* the config key is `utilDirs` (plural, list); singular `utilsDir` is silently ignored, leaving utilities unresolved.

**D2 — Rule subdirs by layer, not by crate.** `rules/{app,entities,infra,shared}/` name the architectural layer a rule governs; the `files:` glob still points at the concrete source path. *Alternative rejected:* per-crate dirs (`rules/orchestrator/`) — they bind the layout to today's single Rust crate and obscure which layer a rule defends. Layer-first generalizes as more crates adopt the same layering.

**D3 — Gate by severity; ast-grep exits non-zero by default.** `ast-grep scan` returns exit 1 on any `error`-severity finding and exit 0 when only `warning` findings remain — no `--error`-style flag is needed. CI adds two steps: `ast-grep scan` (the gate) and `ast-grep test` (fixtures). *Alternative rejected:* a warning-only advisory run — a warning that never blocks just accumulates; severity is the gate control instead.

**D4 — Error only when green; warning for in-flight rules.** A rule ships `error` only after `ast-grep scan` is green with it present. A rule tied to an unfinished refactor ships `warning` with a note naming the condition that flips it to `error`. This keeps every `error` rule one the tree fully satisfies, while still recording and testing aspirational direction. `command-query-dto` is the warning case (77 findings today).

**D5 — `command-query-dto` rule shape.** Match `struct_item` that (a) has a field which is either not `pub` (`not: pattern "pub $F: $T"`) or whose type is not built-in (`has: {field: type, not: {matches: BUILTIN_TYPE}}`), and (b) is `inside` a `source_file` that `has` an `impl Command<Ctx>`/`impl Query<Ctx>` for that struct. The built-in whitelist is the shared util. *Alternative rejected:* inlining the type regex in the rule — duplicated per future type-shape rule; a `utilDirs` utility centralizes it.

**D6 — `BUILTIN_TYPE` as a regex-only global utility.** One util file (`id` + `language` + `rule`), referenced via `matches: BUILTIN_TYPE`. The rule is regex-only (no `kind`) so it matches the type node's text across `primitive_type`/`type_identifier`/`reference_type`/`generic_type` — covering `i64`, `String`, `&str`, and `Option<…>`/`Vec<…>` alike. *Gotcha:* `regex:` must be single-quoted in YAML; double-quoting mangles `\s`/`\b` escapes and the match silently fails. Each global util is its own file — a `utils:` map or a list both error under `utilDirs`.

## Risks / Trade-offs

- A `warning` rule never blocks → intentional; the rule note names the condition that raises it to `error`, and `ast-grep test` keeps it from silently breaking.
- ast-grep / tree-sitter grammar version drift changes matching → pin the npm version in CI; document the pin.
- Double-quoted `regex:` silently fails → always single-quote; the rule's `invalid` fixtures catch a broken regex at `ast-grep test`.
- A new `error` rule with a hidden existing violation reddens CI for everyone → D4 verify-before-add: run `ast-grep scan` locally before raising severity.

## Migration Plan

1. Add `sgconfig.yml` + the `.ast-grep/` tree (the `command-query-dto` rule, its fixture + snapshot, the `BUILTIN_TYPE` util); install a pinned ast-grep in CI.
2. Run `ast-grep scan` (green — exit 0, warnings only) and `ast-grep test` (pass) locally.
3. Add the blocking `ast-grep scan` + `ast-grep test` steps to `ci.yml`.

Rollback is removing `sgconfig.yml` + the `.ast-grep/` tree + the CI steps; no product code depends on it.

## Open Questions

- Which deferred rule to re-seed first once green (import boundaries vs sqlx macro ban vs module visibility)?
- Local pre-push enforcement (a git hook running `ast-grep scan`) — worth it, or is the CI gate enough?

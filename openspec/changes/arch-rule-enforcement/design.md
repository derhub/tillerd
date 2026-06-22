## Context

tillerd is a polyglot monorepo: `crates/` (9 Rust crates incl. `orchestrator`), `apps/` (8, incl. the TS `ui` and the tauri `desktop`), `packages/` (TS `sdk`, `logger`), driven by a Cargo workspace + Turborepo. CI (`.github/workflows/ci.yml`) runs bun lint / check-types / test, the Rust build, and e2e; `autofix.yml` runs autofix.ci. No structural-rule enforcement exists today — architectural conventions live in docs, memory, and reviewers' heads, and regress silently.

The language-native linters can't express the rules that matter here: clippy and the TS linter reason within a file or crate, not across module/layer boundaries. The need is concrete and current — `client-assigned-create-ids` establishes a strict `entities → infra → app` layering in `orchestrator` and needs that boundary mechanically locked. opengrep is the project's chosen single static-analysis tool (see global `tools/opengrep.md`); this change stands up the harness once, monorepo-wide, so any boundary can be guarded — and leaves headroom for semantic/security rules the same tool supports.

Verified locally against the real tree (opengrep 1.23.0): the seed rules parse the orchestrator Rust (202 files, 0 parse errors) and run green (0 findings); structural matching is comment/string-immune; `paths.include` scopes a rule to a directory; `--error` makes findings fail the build.

## Goals / Non-Goals

**Goals:**

- One repo-root `rules/` tree that a single `opengrep scan --error` evaluates across all crates/packages.
- A two-tier rule layout: package-scoped rules by default, shared rules when a rule recurs across packages.
- A blocking CI step that fails the build on any `ERROR`-severity finding.
- Seed with rules already at zero violations, all `ERROR`: the orchestrator layer boundaries and the no-compile-time-query-macro convention.

**Non-Goals:**

- No rules for constructs with existing violations (e.g. `unwrap`/`expect`, ~1223) — no warning tier, no baseline/ratchet machinery in v1.
- No TS boundary rules yet (sdk zero-dep, ui↛engine) — deliberate fast-follow (same tool covers them).
- No semantic/taint or autofix rules yet; this is structural enforcement only (the tool supports both later).
- No cross-repo rule-pack sync — reuse is recipe-level (global `tools/opengrep.md`).

## Decisions

**D1 — opengrep, rules loaded from a directory; no separate project config.** Rule `.yml` files live under a repo-root `rules/` tree; `opengrep scan --error -f rules <paths>` loads every rule recursively (including nested subdirs). No separate project-config file is needed — pointing `-f rules` at the tree is the whole wiring. Verified: rules under `rules/shared/` and `rules/orchestrator/` are all picked up by one invocation.

**D2 — Two-tier rule layout, scope by per-rule `paths.include`; promote by moving + widening.**

```
rules/
  shared/                    cross-crate/package rules   (broad paths.include)
    sqlx-no-query-macro.yml      paths.include: ["crates/"]
  orchestrator/              package-scoped rules        (narrow paths.include)
    infra-no-app.yml             paths.include: ["crates/orchestrator/src/infra/"]
    entities-pure.yml            paths.include: ["crates/orchestrator/src/entities/"]
```

A rule is package-scoped by giving its `paths.include` that package's path. Promotion to shared = move the file into `rules/shared/` and widen its `paths.include`. Verified: the orchestrator-scoped `infra-no-app` flagged only `orchestrator/src/infra` (not `app/`), while the shared query-macro rule flagged a file in a different crate.

**D3 — One repo-root `rules/` tree, not per-crate.** A single tree covers the whole monorepo in one scan and can host cross-package shared rules. Per-crate rule sets were rejected: they can't express a shared rule once, and they multiply scan invocations.

**D4 — Blocking CI step `opengrep scan --error`, opengrep installed via its install script (pinned).** Add a `ci.yml` step that installs a pinned opengrep version (`install.sh`, self-contained binary, no Python) and runs `opengrep scan --error -f rules .` from the repo root. **The `--error` flag is mandatory** — opengrep exits 0 even with findings by default; `--error` makes any finding exit non-zero (verified). opengrep scans git-tracked files by default, which is exactly what CI wants. Pinning the version is what keeps rule semantics stable across runs.

**D5 — Error-only, zero-violation seed; verify-before-add.** A rule is committed at `severity: ERROR` only after `opengrep scan --error` is green against the current tree with that rule present. A rule that would flag existing code is not added until the code is fixed first. No `WARNING`-severity rules and no baseline in v1 — a warning that never blocks just accumulates as noise. This keeps the harness honest: every rule in the tree is one the codebase fully satisfies.

**D6 — Seed set: layer boundaries (orchestrator-scoped) + query-macro ban (shared).** The `entities/infra/app` layering exists only in `orchestrator` today, so its rules are package-scoped under `rules/orchestrator/`; the query-macro convention applies to any crate touching the database library, so it is shared. If a second crate adopts the same layering, the layer rules promote to `rules/shared/` per D2. Verified rule bodies (Semgrep-compatible YAML):

```yaml
# rules/orchestrator/infra-no-app.yml
rules:
  - id: infra-no-app
    languages: [rust]
    severity: ERROR
    message: "Layer violation: infra/ must not depend on app/."
    pattern-either:
      - pattern: use crate::app::$X;
      - pattern: use crate::app::$X as $Y;
    paths:
      include: ["crates/orchestrator/src/infra/"]
```
```yaml
# rules/orchestrator/entities-pure.yml
rules:
  - id: entities-pure
    languages: [rust]
    severity: ERROR
    message: "Layer violation: entities/ must not depend on app/ or infra/."
    pattern-either:
      - pattern: use crate::app::$X;
      - pattern: use crate::app::$X as $Y;
      - pattern: use crate::infra::$X;
      - pattern: use crate::infra::$X as $Y;
    paths:
      include: ["crates/orchestrator/src/entities/"]
```
```yaml
# rules/shared/sqlx-no-query-macro.yml
rules:
  - id: sqlx-no-query-macro
    languages: [rust]
    severity: ERROR
    message: "Use runtime .bind queries, not compile-time query macros."
    pattern-either:
      - pattern: sqlx::query!(...)
      - pattern: sqlx::query_as!(...)
    paths:
      include: ["crates/"]
```

**D7 — Import-based matching; `$X` is single-node; gaps documented.** The layer rules match `use` imports — the way a Rust module depends on another. `use crate::app::$X;` catches multi-segment imports and is comment/string-immune (structural). opengrep's `$X` matches a single node, so deep inline paths (`crate::app::a::b::c()` with no `use`), `pub use`, and grouped `use crate::{app::…}` each need their own `pattern-either` arm; add them if those forms appear. Verified: all three seed rules run with zero parse errors and zero findings on the current tree.

## Risks / Trade-offs

- **Missing `--error` silently disables the gate** → opengrep exits 0 with findings by default; the CI step MUST pass `--error` (called out in tasks and verified).
- **opengrep version drift changes rule behavior** → pin the version installed in CI; document the pin.
- **`$X` single-node coverage gaps** (deep inline paths, `pub use`, grouped imports) → documented; add `pattern-either` arms only if those forms appear (they don't today).
- **A new ERROR rule with a hidden existing violation reddens CI for everyone** → the D5 verify-before-add discipline: run `opengrep scan --error` locally before committing a rule.
- **git-tracked-only scanning** → fine for CI (checkout is git); locally, an untracked new file is skipped until `git add`.
- **Heavier than a syntactic-only tool** → accepted: one tool covering structural + future semantic/security beats running two.

## Migration Plan

1. Install a pinned opengrep in CI (`install.sh`); add the repo-root `rules/shared/` + `rules/orchestrator/` tree with the three seed rules.
2. Run `opengrep scan --error -f rules crates` locally — confirm green (verified: 202 files, 0 findings).
3. Add the blocking `opengrep scan --error -f rules .` step to `ci.yml`.
4. (Already done in planning) `client-assigned-create-ids` no longer carries the one-off arch test; it relies on this harness.

Rollback is removal of the rules + CI step; nothing in product code depends on it.

## Open Questions

- TS boundary rules (sdk zero-dep, ui↛engine internals) — which exact boundaries, and are they at zero violations today? (fast-follow; `languages: [typescript]`)
- Local pre-push enforcement (a git hook running `opengrep scan --error`) — worth it, or is the CI gate enough?
- SARIF upload (`--sarif-output`) to GitHub code-scanning — surface findings in the PR UI, or keep the plain CI gate?
- Promotion convention — is "move file to `rules/shared/` + widen `paths.include`" enough as a documented step, or does it want a checklist?

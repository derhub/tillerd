## Why

Architectural rules — layer boundaries, banned constructs, structural conventions — are enforced today only by humans, docs, and memory, so they regress silently. The language-native linters can't express them: clippy and the TS linter reason within a file or crate, not across module/layer boundaries. opengrep (the chosen single static-analysis tool) gives structural, declarative rules with a CI exit code, so a violation fails the build instead of slipping through review — and the same tool covers semantic/security rules later.

The trigger is concrete: the `client-assigned-create-ids` change establishes a strict `entities → infra → app` layering and currently carries its own one-off arch test (design D7). That enforcement is reusable infrastructure, not a detail of one refactor. Extracting it lets the harness land first, be shared across the monorepo, and be consumed by any change that needs to lock a boundary.

## What Changes

- Add opengrep to CI as a dedicated, blocking `scan --error` step (pinned version); `--error` makes any finding a non-zero exit that fails the job.
- Add a repo-root `rules/` tree, two-tier by scope:
  - **package-scoped** rules apply to one crate/package via a `paths.include` glob;
  - **shared** rules are promoted into a shared rule dir when the same rule recurs across crates/packages.
- Seed v1 with rules that are already at **zero violations**, enforced as `severity: ERROR`:
  - **layer boundaries** (Rust): `infra/` must not import `app/`; `entities/` must not import `app/` or `infra/`;
  - **sqlx guard** (Rust): ban `query!` / `query_as!` macros (repo is runtime-`.bind` only).
- **Adoption policy:** only rules already at zero violations ship in v1, all as `ERROR`. Rules with existing violations (e.g. `unwrap`/`expect`, ~1223 occurrences) are explicitly out of scope — no warning-severity rules, no baseline/ratchet machinery in v1. There is no baseline mechanism in v1; adding one is a separate, later decision.
- **Supersedes** the one-off arch test in `client-assigned-create-ids` (D7 + tasks 5.1–5.2): those drop, and that change relies on this harness to satisfy its "layer dependency rules are enforced automatically" requirement.

## Capabilities

### New Capabilities

- `architecture-rule-enforcement`: the monorepo declares structural/architectural rules as opengrep rules; a blocking CI check fails the build on any `ERROR`-severity finding, reporting the offending file. Rules are scoped to a package by default and promoted to a shared scope when they apply across packages. Only rules at zero existing violations are enforced as errors.

### Modified Capabilities

<!-- None. `domain-model-boundary` (in client-assigned-create-ids) declares the layer rules
     must be enforced automatically; this change provides the enforcement mechanism and the
     concrete layer rules. The two compose — no requirement text in domain-model-boundary changes. -->

## Impact

- **New files:** repo-root `rules/` tree (`rules/shared/` + per-package dirs, e.g. `rules/orchestrator/`) holding the layer rules and the query-macro rule.
- **CI:** `.github/workflows/ci.yml` gains an opengrep install + `opengrep scan --error` step that blocks on any finding. Pin the opengrep version.
- **`client-assigned-create-ids`:** design D7 and tasks 5.1–5.2 are removed; its `domain-model-boundary` enforcement requirement is satisfied by this harness. (Needs a follow-up edit to that change.)
- **Reuse:** rules are per-repo; the package→shared promotion path is documented. Cross-repo sharing is recipe-level only (global `tools/opengrep.md`), not a synced rule-pack — out of scope here.
- **Scope of enforcement at v1:** Rust crates only. TS boundary rules (sdk zero-dep, ui↛engine internals) are a deliberate fast-follow, not v1.

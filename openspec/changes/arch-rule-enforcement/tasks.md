## 1. Harness

- [ ] 1.1 Install a pinned opengrep in CI and document the version (via `install.sh`, self-contained binary; record the pin).
- [ ] 1.2 Create the repo-root `rules/` tree: `rules/shared/` and `rules/orchestrator/`.

## 2. Seed rules (Semgrep YAML, `severity: ERROR`, verified green today)

- [ ] 2.1 `rules/orchestrator/infra-no-app.yml` — `paths.include: ["crates/orchestrator/src/infra/"]`, `pattern-either: [use crate::app::$X;, use crate::app::$X as $Y;]`.
- [ ] 2.2 `rules/orchestrator/entities-pure.yml` — `paths.include: ["crates/orchestrator/src/entities/"]`, `pattern-either` of `use crate::app::$X;` / `use crate::infra::$X;` (+ the `as $Y;` variants).
- [ ] 2.3 `rules/shared/sqlx-no-query-macro.yml` — `paths.include: ["crates/"]`, `pattern-either: [sqlx::query!(...), sqlx::query_as!(...)]`.
- [ ] 2.4 Run `opengrep scan --error -f rules crates` from the repo root; confirm 0 findings (green) on the current tree.

## 3. CI gate

- [ ] 3.1 Add a blocking step to `.github/workflows/ci.yml`: install pinned opengrep, then `opengrep scan --error -f rules .` from the repo root. **`--error` is required** — opengrep exits 0 even with findings without it.

## 4. Verify

- [ ] 4.1 Run `opengrep scan --error -f rules crates` (must pass), then add a temporary `use crate::app::…` in `orchestrator/src/infra` and a `sqlx::query!` in any crate to confirm the scan and the CI step both exit non-zero; revert the temporaries.

## Why

Structural code conventions — like "a command/query DTO carries only plain built-in fields" — are enforced today only by reviewers and memory, so they regress silently. The language-native linters can't express them: clippy reasons about types and lints, not about a struct's role in the CQS layer. ast-grep gives structural, declarative rules with a CI exit code, and already backs structural code search — so one tool covers both search and enforcement.

## What Changes

- Add ast-grep as the project's static-analysis enforcement tool: a repo-root `sgconfig.yml` (`ruleDirs: [.ast-grep/rules]`, `testConfigs: [{testDir: .ast-grep/tests}]`, `utilDirs: [.ast-grep/utils]`) plus the `.ast-grep/` tree.
- Organize rule files **by layer** (`rules/app/`, `entities/`, `infra/`, `shared/`); each rule's `files:` glob points at the real source path. Every rule carries a `valid:`/`invalid:` test fixture + snapshot under `.ast-grep/tests`.
- Seed one rule, `app/command-query-dto`: a struct that `impl`s `Command<Ctx>`/`Query<Ctx>` must have all fields `pub` and of plain built-in type (String, integers, bool, char, `&str`, or `Option`/`Vec` of those); domain newtypes like `ProjectId` are violations, converted inside the handler. It ships `severity: warning` (77 findings on the current tree) until the DTOs are migrated to primitives, then flips to `error`.
- Add a reusable `BUILTIN_TYPE` utility rule under `utilDirs`, referenced via `matches:`, defining the built-in type whitelist once for reuse by future type-shape rules.
- Wire CI: a dedicated job installs a pinned ast-grep (`npm install -g @ast-grep/cli@<pin>`) and runs `ast-grep scan` (gates by default — exit 1 on any `error`-severity finding, exit 0 when only warnings remain) plus `ast-grep test`.

## Capabilities

### New Capabilities

- `code-rule-enforcement`: the repo declares structural code rules as ast-grep rules under a layer-organized `.ast-grep/` tree; a blocking CI check (`ast-grep scan`) fails the build on any `error`-severity finding and `ast-grep test` runs each rule's fixtures. Rules ship `error` only when the tree is already green; rules tied to an in-flight refactor ship `warning` until it lands. Shared sub-rules live as `utilDirs` utilities referenced via `matches:`. The seed rule is `command-query-dto`.

### Modified Capabilities

<!-- None. No existing spec's requirements change; this adds an enforcement harness. -->

## Impact

- **New files:** repo-root `sgconfig.yml`; `.ast-grep/rules/app/command-query-dto.yaml`; `.ast-grep/utils/rust-types.yml` (`BUILTIN_TYPE`); `.ast-grep/tests/app/command-query-dto.test.yml` + snapshot.
- **CI:** `.github/workflows/ci.yml` gains an `arch` job (install pinned ast-grep, `ast-grep scan`, `ast-grep test`). Pin the ast-grep version.
- **Tooling:** ast-grep becomes the single enforcement tool; the recipe lives at global `tools/ast-grep.md`. No product code changes.
- **Follow-up (out of scope here):** migrate CQS DTOs from domain newtypes to primitives so `command-query-dto` flips to `error`; add further layer rules (import boundaries, handler hygiene, sqlx macro ban) once re-seeded green.

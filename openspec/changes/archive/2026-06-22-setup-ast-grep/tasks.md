## 1. Harness

- [x] 1.1 Add repo-root `sgconfig.yml` (`ruleDirs: [.ast-grep/rules]`, `testConfigs: [{testDir: .ast-grep/tests}]`, `utilDirs: [.ast-grep/utils]`) and the `.ast-grep/{rules,tests,utils}` tree with layer subdirs.
- [x] 1.2 Add the `BUILTIN_TYPE` global utility (`.ast-grep/utils/rust-types.yml`, regex-only, single-quoted) referenced via `matches:`.

## 2. Seed rule

- [x] 2.1 Add `.ast-grep/rules/app/command-query-dto.yaml` (`severity: warning`): struct impl'ing `Command<Ctx>`/`Query<Ctx>` with a non-`pub` field or a field whose type is `not: { matches: BUILTIN_TYPE }`.
- [x] 2.2 Add `.ast-grep/tests/app/command-query-dto.test.yml` valid/invalid fixtures + snapshot; `ast-grep test` passes.

## 3. CI gate

- [x] 3.1 Add a CI job to `.github/workflows/ci.yml`: install pinned `@ast-grep/cli`, run `ast-grep scan` (gates by default) and `ast-grep test`.

## 4. Verify

- [x] 4.1 Run `ast-grep scan` (exit 0, warnings only) and `ast-grep test` (pass); temporarily add a non-built-in field to a `Command<Ctx>` DTO and confirm it surfaces as a `warning`, then revert.

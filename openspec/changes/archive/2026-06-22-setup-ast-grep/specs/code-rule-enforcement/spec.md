## ADDED Requirements

### Requirement: Declarative rule harness

The repository SHALL declare structural code rules as ast-grep rule files loaded by a repo-root `sgconfig.yml` (`ruleDirs`, `testConfigs`, `utilDirs`). Rule files SHALL be organized into subdirectories named for the architectural layer the rule governs (`app`, `entities`, `infra`, `shared`), and each rule's `files:` glob SHALL scope it to the real source path it applies to.

#### Scenario: Rules load from the configured tree

- **WHEN** `ast-grep scan` runs from the repo root
- **THEN** every rule file under `ruleDirs` is evaluated without naming any rule or path on the command line

#### Scenario: A rule applies only within its scope

- **WHEN** a rule declares `files: ["crates/orchestrator/src/app/**"]`
- **THEN** the rule is evaluated only against files under that path and ignores matches elsewhere

### Requirement: Blocking CI gate

CI SHALL run `ast-grep scan` as a blocking step that fails the build on any `error`-severity finding and passes when only `warning`-severity findings remain, and SHALL run `ast-grep test` so every rule's fixtures are exercised. The installed ast-grep version SHALL be pinned.

#### Scenario: Error finding fails the build

- **WHEN** the tree contains a node matching an `error`-severity rule
- **THEN** `ast-grep scan` exits non-zero and the CI job fails

#### Scenario: Warning finding does not fail the build

- **WHEN** the only findings are `warning`-severity
- **THEN** `ast-grep scan` exits zero and the CI job passes

#### Scenario: Rule tests run in CI

- **WHEN** the CI job runs
- **THEN** `ast-grep test` executes each rule's `valid`/`invalid` fixtures and fails the build if any fixture's expectation is not met

### Requirement: Severity adoption policy

A rule SHALL ship at `severity: error` only when the current tree already satisfies it (zero findings). A rule tied to an in-flight refactor SHALL ship at `severity: warning` until that refactor makes the tree green, at which point it is raised to `error`.

#### Scenario: Aspirational rule does not block

- **WHEN** a rule still has outstanding violations because its refactor has not landed
- **THEN** the rule is declared `severity: warning` and CI remains green while still reporting the findings

### Requirement: Reusable utility rules

Shared sub-rules SHALL be defined as ast-grep utility rules under `utilDirs` (one util per file, shaped as `id` + `language` + `rule`) and referenced from rules via `matches: <id>`.

#### Scenario: A rule reuses a shared utility

- **WHEN** a rule references `matches: BUILTIN_TYPE`
- **THEN** the rule matches exactly when the `BUILTIN_TYPE` utility rule matches, with the definition maintained in one place

### Requirement: Command/Query DTO field shape

The seed rule `command-query-dto` SHALL flag any struct that implements `Command<Ctx>` or `Query<Ctx>` and has a field that is either not `pub` or not a plain built-in type (a primitive, `String`, `&str`, or a single-level `Option`/`Vec` of those). It SHALL ship `severity: warning` until the existing DTOs are migrated off domain newtypes.

#### Scenario: Compliant DTO passes

- **WHEN** a `Command<Ctx>`/`Query<Ctx>` struct has every field `pub` and built-in-typed
- **THEN** the rule reports no finding for that struct

#### Scenario: Domain-newtype field is flagged

- **WHEN** a `Command<Ctx>`/`Query<Ctx>` struct has a field typed as a domain newtype such as `ProjectId`
- **THEN** the rule reports a `warning` finding on that struct

#### Scenario: Private field is flagged

- **WHEN** a `Command<Ctx>`/`Query<Ctx>` struct has a non-`pub` field
- **THEN** the rule reports a `warning` finding on that struct

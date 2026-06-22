# Code Rule Enforcement

## Purpose

Enforce architectural and code quality rules declaratively via ast-grep, with blocking CI enforcement and severity-based adoption policies.

## Requirements

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

### Requirement: No compile-time sqlx query macros

The project uses runtime sqlx with `.bind`, never the compile-time `query!`/`query_as!` macros (no `.sqlx`/`DATABASE_URL` build dependency). The rule `sqlx-no-query-macro` SHALL flag any `query!` or `query_as!` macro invocation (qualified or bare) under `crates/`. The tree satisfies this today, so it ships `severity: error` (gating).

#### Scenario: A query macro is flagged

- **WHEN** a file invokes `sqlx::query!(…)` or `query_as!(…)`
- **THEN** the rule reports an `error` finding and the CI scan fails

#### Scenario: Runtime .bind queries pass

- **WHEN** a file uses `sqlx::query_as::<_, Row>("…").bind(x)`
- **THEN** the rule reports no finding

### Requirement: Entities layer purity

The entities layer is the pure domain model and MUST depend on neither app/ nor infra/ (dependency direction is entities ← infra ← app). The rule `entities-pure` SHALL flag any `use crate::app…` or `use crate::infra…` import under `crates/orchestrator/src/entities/`. The tree satisfies this today, so it ships `severity: error` (gating).

#### Scenario: Entity importing app or infra is flagged

- **WHEN** a file under `entities/` declares `use crate::app::…` or `use crate::infra::…`
- **THEN** the rule reports an `error` finding and the CI scan fails

#### Scenario: Entity importing shared or another entity passes

- **WHEN** a file under `entities/` imports `crate::shared::…` or `crate::entities::…`
- **THEN** the rule reports no finding

### Requirement: Aggregate entities are sqlx-mappable

An aggregate entity (a brace struct under `entities/` that maps to a table row) SHOULD derive `sqlx::FromRow` so a repository queries straight into it — no intermediate `*Row` struct and no second map (derived columns are computed in the SELECT). The rule `aggregate-entity-fromrow` SHALL flag any brace struct under `entities/` that is not a `New*` input DTO and is not preceded by a derive containing `FromRow` or `Serialize`. It ships `severity: warning` until the aggregates derive `FromRow`.

#### Scenario: Aggregate without FromRow is flagged

- **WHEN** `entities/` defines `pub struct Project { … }` with no `FromRow` derive
- **THEN** the rule reports a `warning` finding

#### Scenario: A serde JSON value object is skipped

- **WHEN** a brace struct derives `Serialize`/`Deserialize` (a JSON-embedded value object, not a table row)
- **THEN** the rule reports no finding

#### Scenario: A New* input DTO is skipped

- **WHEN** the brace struct's name begins with `New`
- **THEN** the rule reports no finding (it is covered by `entities-no-input-dto`)

### Requirement: Value objects are sqlx-mappable

A value object (a newtype/tuple struct under `entities/`) SHOULD derive `sqlx::Type` with `#[sqlx(transparent)]`, so a repository binds and decodes it directly against its column without manual wrapping. The rule `value-object-sqlx-type` SHALL flag any tuple-struct under `entities/` not preceded by a derive containing `Type`; brace-struct aggregate entities are out of scope. It ships `severity: warning` until the value objects derive `sqlx::Type`.

#### Scenario: Newtype without sqlx::Type is flagged

- **WHEN** `entities/` defines `pub struct ProjectId(String);` with no `sqlx::Type` derive
- **THEN** the rule reports a `warning` finding

#### Scenario: Newtype with sqlx::Type passes

- **WHEN** the newtype is preceded by `#[derive(sqlx::Type)] #[sqlx(transparent)]`
- **THEN** the rule reports no finding

#### Scenario: Aggregate entity is out of scope

- **WHEN** `entities/` defines a brace struct `pub struct Project { … }`
- **THEN** the rule reports no finding (it maps via the repo's row→entity step)

### Requirement: Entities hold no input DTOs

The entities layer holds domain entities and value objects only — not create-input DTOs. The rule `entities-no-input-dto` SHALL flag any struct under `entities/` whose name begins with `New` (the create-input naming convention); such a shape belongs on the command in app/. It ships `severity: warning` until the `New*` types are moved out. This is a naming heuristic, not a semantic entity/DTO classifier.

#### Scenario: A New* struct in entities is flagged

- **WHEN** `entities/` defines `pub struct NewProject { … }`
- **THEN** the rule reports a `warning` finding

#### Scenario: A domain entity or value object passes

- **WHEN** `entities/` defines `pub struct Project { … }` or `pub struct ProjectId(String)`
- **THEN** the rule reports no finding

### Requirement: Query returns a read DTO

A query SHALL return a read model, not the write model: its `Query::Out` MUST be a read DTO named `*View` (flat, `Serialize` + `FromRow`, mapped straight from the row via `query_as`) or a primitive scalar (an integer, `bool`, `char`, `String`, `&str`, or an `Option`/`Vec`/`Listing` of those). It MUST NOT be a domain entity or value object. The rule `query-returns-view` SHALL flag any `type Out = …` inside an `impl Query<Ctx>` whose type is neither a `*View` nor a primitive, and SHALL ship `severity: warning` until every query returns a read DTO.

#### Scenario: Query returning a view or primitive passes

- **WHEN** a query declares `type Out = Option<ProjectView>;` or `type Out = i64;`
- **THEN** the rule reports no finding

#### Scenario: Query returning an entity is flagged

- **WHEN** a query declares `type Out = Option<Project>;` or `type Out = Listing<Session>;`
- **THEN** the rule reports a `warning` finding on that `Out`

### Requirement: Message DTO field shape

A message DTO is the struct for a CQS message — a `Command`, `Query`, or `Io`. The seed rule `message-dto` SHALL flag any struct that implements `Command<Ctx>`, `Query<Ctx>`, or `Io<Ctx>` and has a field that is either not `pub` or not a plain built-in type (a primitive, `String`, `&str`, or a single-level `Option`/`Vec` of those). It SHALL ship `severity: warning` until the existing DTOs are migrated off domain newtypes.

#### Scenario: Compliant DTO passes

- **WHEN** a `Command<Ctx>`/`Query<Ctx>` struct has every field `pub` and built-in-typed
- **THEN** the rule reports no finding for that struct

#### Scenario: Domain-newtype field is flagged

- **WHEN** a `Command<Ctx>`/`Query<Ctx>` struct has a field typed as a domain newtype such as `ProjectId`
- **THEN** the rule reports a `warning` finding on that struct

#### Scenario: Private field is flagged

- **WHEN** a `Command<Ctx>`/`Query<Ctx>` struct has a non-`pub` field
- **THEN** the rule reports a `warning` finding on that struct

### Requirement: Message DTO deserializability

A message DTO SHALL derive `Deserialize` so the host transport can bind an invoke payload straight into it, removing per-command argument plumbing. The rule `message-dto-deserialize` SHALL flag any struct that implements `Command<Ctx>`, `Query<Ctx>`, or `Io<Ctx>` and is not preceded by a `#[derive(… Deserialize …)]` attribute. It SHALL ship `severity: warning` until the existing message DTOs derive `Deserialize`.

#### Scenario: DTO deriving Deserialize passes

- **WHEN** a message DTO is preceded by `#[derive(Deserialize)]`, including when a `#[serde(...)]` attribute sits between the derive and the struct
- **THEN** the rule reports no finding for that struct

#### Scenario: DTO without Deserialize is flagged

- **WHEN** a message DTO has no `Deserialize` in its derive list (or no derive at all)
- **THEN** the rule reports a `warning` finding on that struct

### Requirement: Entity crate boundary

Domain value objects MUST NOT leak outside the orchestrator crate: no consumer crate SHALL import `orchestrator::entities`. The boundary is crossed with primitives — flat input DTOs, `Serialize` read DTOs as `Query::Out`, and ports that speak primitives. The rule `entities-stay-internal` SHALL flag any `use orchestrator::entities…` import in any crate other than `orchestrator` itself, and SHALL ship `severity: warning` until no consumer imports entities.

#### Scenario: Consumer importing an entity is flagged

- **WHEN** a file outside the orchestrator crate declares `use orchestrator::entities::ProjectId;` (named, glob, or alias)
- **THEN** the rule reports a `warning` finding

#### Scenario: Consumer importing app or shared types passes

- **WHEN** a consumer imports `orchestrator::app::…` or `orchestrator::shared::…`
- **THEN** the rule reports no finding

#### Scenario: Orchestrator-internal entity use is ignored

- **WHEN** a file inside the orchestrator crate uses `crate::entities::…`
- **THEN** the rule reports no finding (the crate is excluded)

### Requirement: Infra crate boundary

Infra — repos, adapters, and the runtime port — MUST stay internal to the orchestrator crate: no consumer crate SHALL import `orchestrator::infra`. The host gets what it needs through a public edge that `app/` owns (a command, query, or public port), not by reaching into the adapter. The rule `infra-stays-internal` SHALL flag any `use orchestrator::infra…` import in any crate other than `orchestrator` itself, and SHALL ship `severity: warning` until no consumer imports infra.

#### Scenario: Consumer importing infra is flagged

- **WHEN** a file outside the orchestrator crate declares `use orchestrator::infra::runtime::FakeRuntime;` (named, glob, or alias)
- **THEN** the rule reports a `warning` finding

#### Scenario: Orchestrator-internal infra use is ignored

- **WHEN** a file inside the orchestrator crate uses `crate::infra::…`
- **THEN** the rule reports no finding (the crate is excluded)

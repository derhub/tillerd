# infra-raw-boundary Specification

## Purpose
TBD - created by archiving change infra-raw-app-owns-domain. Update Purpose after archive.
## Requirements
### Requirement: Infra is a raw API

The infrastructure layer SHALL contain only raw operations: executing and binding database statements, mapping columns to and from entity fields, opening and reading and writing sockets, encoding and decoding wire frames, and reading and writing files. It SHALL NOT contain domain rules — no business invariants or guards, no retention or precedence policies, no initial-state or ordering decisions, no capability rules, and no multi-step "load, apply a rule, persist" sequences.

#### Scenario: A repository exposes bare operations

- **WHEN** an app handler needs stored data
- **THEN** infra offers a bare get/list/create/update/delete that runs a query and maps columns, with no decision about whether the operation is allowed

#### Scenario: A guard is rejected from infra

- **WHEN** a domain invariant (for example "a prebuilt item cannot be removed") must be enforced
- **THEN** the invariant lives in an app handler, and infra performs the raw delete without checking it

### Requirement: App owns all domain logic

The app layer SHALL own every domain rule and SHALL be the sole integrator of entities and infra: a use-case handler loads through raw infra, applies entity rules, and persists through raw infra. Domain decisions — precedence, retention, capability, normalization, initial state, ordering — SHALL be visible in the handler (or in an entity method it calls), not hidden in a store.

#### Scenario: A precedence rule is read in the handler

- **WHEN** an effective value is resolved from project and global scopes
- **THEN** the app handler performs the project-over-global cascade using raw per-scope reads, so the precedence is stated where domain rules live

#### Scenario: A capability rule is enforced before the effect

- **WHEN** a use case may run only for a permitted kind or state
- **THEN** the app handler checks the rule before any persistence or external effect, so no partial state is written for a rejected request

### Requirement: Infra exposes concrete raw types

Infra SHALL expose its raw operations as concrete types named by app, not as a port/trait abstraction over a single I/O target. Where app needs a test double, the swap SHALL use static dispatch (an enum over the concrete prod and fake types) rather than a `dyn` trait object. A trait is justified only at a genuine multi-implementation output boundary (for example a `SurfaceEventSink` with more than one real renderer), not as an indirection over one concrete client.

#### Scenario: The surface runtime is a concrete client

- **WHEN** the app drives a surface's pseudo-terminal
- **THEN** it calls a concrete `DaemonPtyApi` (the raw daemon-socket client) held by the composition root, with the prod/fake choice resolved by an enum at construction, not through an `Arc<dyn>` port

### Requirement: The boundary is enforced by lint

The raw/domain split SHALL be enforced by the structural rules `entities-app-or-infra-only` and `infra-only-in-app`. The bootstrap (`boot.rs` and `context.rs`, the composition root) may name any layer. Otherwise: code outside `app/` and the bootstrap SHALL NOT name `crate::infra`, and code outside `app/`, `infra/`, and the bootstrap SHALL NOT name `crate::entities` (infra may name entities only for `Row <-> Entity` column mapping). The rules ship `severity: warning` until the leaks are removed, then flip to `error`.

#### Scenario: A non-app reference is flagged

- **WHEN** a file outside `app/` references `crate::entities` or `crate::infra`
- **THEN** the scan reports it, and once the tree is clean the rule is raised to error so a regression fails the build


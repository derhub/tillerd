## ADDED Requirements

### Requirement: The end-to-end suite provisions its own dependencies

The end-to-end test suite SHALL start, on its own, every runtime dependency it needs — including the terminal daemon — against an isolated temporary runtime directory, and SHALL tear them down afterward. It SHALL NOT require a pre-running service or any manually exported environment.

#### Scenario: Runs on a clean checkout with nothing pre-started

- **WHEN** the end-to-end suite runs on a clean checkout with no service already running and no environment exported
- **THEN** it SHALL start the dependencies it needs, run its tests, and pass

#### Scenario: Uses an isolated runtime directory

- **WHEN** the end-to-end suite runs
- **THEN** it SHALL use a temporary runtime directory distinct from the developer's, and remove it when finished

#### Scenario: Stops what it started

- **WHEN** the end-to-end suite finishes, whether it passes or fails
- **THEN** every dependency process it started SHALL be stopped, leaving no orphan

### Requirement: One command runs the whole battery

A single verification command SHALL run the complete battery — type-checking, linting, unit and integration tests, and the end-to-end suite — and report one pass/fail outcome.

#### Scenario: Whole battery in one invocation

- **WHEN** the verification command runs
- **THEN** it SHALL execute a format check, type-checking, linting, the test suite, and the end-to-end suite, and SHALL fail if any one of them fails

### Requirement: Continuous integration runs verification on every change

Continuous integration SHALL run the verification command on every push and pull request, so a regression in any suite is caught before merge rather than during a later manual run.

#### Scenario: Verification runs in CI

- **WHEN** a change is pushed or a pull request is opened
- **THEN** CI SHALL run the full verification command and report its result as the build status

### Requirement: Formatting, type-checking, and linting pass on a clean checkout

A format check SHALL be available as a non-mutating, fail-on-drift command covering both languages, and it SHALL pass on a clean checkout. Type-checking SHALL pass for all application code on a clean checkout; a file that is a runtime launcher outside the application's type configuration SHALL be excluded from that type-check rather than left to fail it. Linting SHALL pass on a clean checkout; a module that is intentionally not yet wired SHALL be annotated so dead-code analysis does not fail the build, and the wiring SHALL be tracked as a follow-up.

#### Scenario: Format check is clean and non-mutating

- **WHEN** the format check runs on a clean checkout
- **THEN** it SHALL report no formatting drift and SHALL NOT modify any file

#### Scenario: Type-check is clean

- **WHEN** type-checking runs on a clean checkout
- **THEN** it SHALL report no errors

#### Scenario: Lint is clean

- **WHEN** linting runs on a clean checkout
- **THEN** it SHALL report no errors, including no dead-code error for an intentionally-unwired module

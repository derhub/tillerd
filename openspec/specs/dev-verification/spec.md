# dev-verification Specification

## Purpose

The dev test battery is self-provisioning and regression-proof: each suite starts its own runtime dependencies (no manual environment), a single command runs the whole battery, and continuous integration runs it on every change so a clean checkout stays green.

## Requirements

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

### Requirement: Desktop end-to-end suite covers the working app

The desktop E2E suite SHALL cover boot-to-ready in dev and bundled modes, full project
and session create flows, and resume after an app restart. The suite SHALL run in CI.
Later milestones SHALL verify desktop behavior by extending this suite, not by manual
checks.

#### Scenario: Boot to ready in both modes

- **WHEN** the suite runs against a dev build and against a bundled build
- **THEN** each boots to a ready shell and the boot spec passes in both modes

#### Scenario: Create flows and resume covered

- **WHEN** the suite runs
- **THEN** it exercises project create, session create, and resume-after-restart
  end to end through the real UI

#### Scenario: Suite runs in CI

- **WHEN** a change lands
- **THEN** CI executes the desktop E2E suite and fails the change on a failing spec

### Requirement: Desktop IPC commands are contract-tested at runtime

Every desktop IPC command SHALL be invoked at runtime by a contract test that asserts
the command is registered and its argument shape deserializes — the response error is
never command-not-found or deserialization failure.

#### Scenario: Arg-shape drift fails the test

- **WHEN** a command's argument struct drifts from what the UI sends
- **THEN** the contract test fails on that command's deserialization error

### Requirement: The e2e scenario suite shares one app instance across the run

The desktop e2e scenario tests SHALL run against a single app instance launched once for the
whole run, rather than cold-booting per test. The instance SHALL be created by a global setup
before any scenario test and torn down once after the last, leaving no orphan process or
service. Before each scenario test the suite SHALL return the shared app to a known baseline —
no open overlay (context menu, dialog, inline-rename input, or command palette) and the home
route mounted — so scenario tests remain order-independent.

Reuse SHALL NOT weaken the existing self-provisioning, isolated-runtime, or no-orphan
guarantees, and SHALL preserve the dev boot-to-ready, bundled boot-to-ready, and
resume-after-restart coverage.

#### Scenario: One launch serves the whole scenario suite

- **WHEN** the e2e scenario suite runs
- **THEN** the app boots to ready once and every scenario test runs against that one instance

#### Scenario: The shared launch is the dev boot-to-ready assertion

- **WHEN** the shared app is launched for the run
- **THEN** reaching the ready shell SHALL satisfy the dev-mode boot-to-ready check, with no
  separate dev-mode boot launch

#### Scenario: Scenario tests stay order-independent

- **WHEN** the scenario tests run in any order against the shared app
- **THEN** each starts from the home route with no overlay left open by a prior test, and passes

#### Scenario: Lifecycle specs keep their own launches

- **WHEN** the suite runs
- **THEN** resume-after-restart and bundled boot-to-ready run as their own launches outside the
  shared-app run, so a scenario-suite failure cannot mask them

#### Scenario: Created entities do not collide across back-to-back tests

- **WHEN** consecutive scenario tests create projects or sessions without a boot between them
- **THEN** each created entity has a name unique within the run, so name-targeted lookups never
  match a prior test's entity

#### Scenario: Shared app is reaped when the run ends

- **WHEN** the scenario run finishes, whether passing or failing
- **THEN** the shared app process, its services, and its webdriver session SHALL be torn down,
  leaving no orphan

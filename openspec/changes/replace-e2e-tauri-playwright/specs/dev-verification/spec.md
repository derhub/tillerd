## ADDED Requirements

### Requirement: E2E automation instrumentation stays out of production builds

The desktop application SHALL compile its e2e automation instrumentation (the in-app bridge that lets the test driver evaluate scripts and observe the webview) only when an explicit test-oriented build feature is enabled. A production or release build without that feature SHALL contain no automation bridge, open no automation endpoint, and grant no automation permission.

#### Scenario: Release build has no automation surface

- **WHEN** the desktop application is built without the e2e test feature
- **THEN** the automation bridge is not compiled in and no automation endpoint is listening at runtime

#### Scenario: E2E build exposes the bridge only to the local driver

- **WHEN** the desktop application is built with the e2e test feature and launched by the test harness
- **THEN** the automation bridge accepts connections only from the local test driver

## MODIFIED Requirements

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
- **THEN** the shared app process, its services, and its automation session SHALL be torn down,
  leaving no orphan

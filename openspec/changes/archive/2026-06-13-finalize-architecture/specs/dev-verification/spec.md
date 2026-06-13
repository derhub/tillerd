## ADDED Requirements

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

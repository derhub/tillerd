## ADDED Requirements

### Requirement: Architectural rules are declared as structural, machine-checkable rules

The repository SHALL express its structural and architectural rules in a declarative form that is evaluated against the source's syntax tree, not its raw text. A rule SHALL match code by structure, so an occurrence of a forbidden construct inside a comment or string literal SHALL NOT count as a violation.

#### Scenario: A real forbidden construct is flagged

- **WHEN** a source file contains the construct a rule forbids, in code
- **THEN** the check reports it as a violation with the file and location

#### Scenario: The same text inside a comment or string is not flagged

- **WHEN** the forbidden construct appears only inside a comment or a string literal
- **THEN** the check does not report it as a violation

### Requirement: A blocking continuous-integration check fails the build on any error-severity violation

Continuous integration SHALL run the rule check on every proposed change. Any rule violation at error severity SHALL fail the build and identify the offending file. The check SHALL NOT require compiling or executing the code under inspection.

#### Scenario: An error-severity violation fails the build

- **WHEN** a change introduces a file that violates an error-severity rule
- **THEN** the continuous-integration check fails and names the offending file

#### Scenario: A clean tree passes

- **WHEN** a change introduces no violation of any rule
- **THEN** the check passes

### Requirement: A rule enforced at error severity has zero existing violations when introduced

A rule MAY be introduced at build-failing (error) severity only when the codebase has no existing violation of it. A rule whose enforcement would require pre-existing code to change first SHALL NOT be added at error severity in the same step; that cleanup is a separate effort, and until it is done the rule is not enforced as an error.

#### Scenario: An already-satisfied rule is added as an error and stays green

- **WHEN** a rule is introduced at error severity and the current tree already satisfies it
- **THEN** the check passes on the unchanged tree and fails only on a future violation

#### Scenario: A rule with existing violations is not introduced as an error

- **WHEN** a candidate rule would be violated by code already in the tree
- **THEN** it is not added at error severity until those violations are resolved

### Requirement: Rules are scoped to the narrowest unit and promoted when shared

A rule SHALL be scoped to the narrowest unit it applies to — by default a single package or crate. When the same rule applies across multiple packages, it SHALL be promoted to a shared scope and defined once, rather than duplicated per package.

#### Scenario: A package-specific rule applies only within its package

- **WHEN** a rule is meaningful only for one package
- **THEN** it is scoped to that package and does not flag identical code in other packages

#### Scenario: A rule shared across packages is defined once at shared scope

- **WHEN** the same rule is required by two or more packages
- **THEN** it is defined a single time at a shared scope rather than copied into each package

### Requirement: The initial enforced rule set guards the layer boundaries and the query-construction convention

At introduction the enforced rule set SHALL include both: the layer-dependency boundaries — the persistence layer SHALL NOT depend on the application layer, and the domain-model layer SHALL NOT depend on the application or persistence layers (the application layer MAY depend on both); and the database-query convention — queries SHALL be constructed by runtime parameter binding rather than compile-time-checked query macros.

#### Scenario: The persistence layer importing the application layer fails

- **WHEN** a persistence-layer file references the application layer
- **THEN** the check fails and names the file

#### Scenario: The domain-model layer importing another layer fails

- **WHEN** a domain-model file references the application or persistence layer
- **THEN** the check fails and names the file

#### Scenario: A compile-time query macro fails

- **WHEN** a file constructs a query through a compile-time-checked query macro
- **THEN** the check fails and names the file

#### Scenario: The application layer importing the persistence layer passes

- **WHEN** an application-layer file references the persistence layer
- **THEN** the check passes

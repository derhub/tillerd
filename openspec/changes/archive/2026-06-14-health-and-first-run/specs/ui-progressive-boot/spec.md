## ADDED Requirements

### Requirement: The shell renders before services are ready

The interface SHALL render the application shell immediately on launch, without
waiting for services to reach a ready state.

#### Scenario: Shell visible during boot

- **WHEN** the application launches and services are still starting
- **THEN** the application shell is rendered
- **AND** no blocking wait screen is shown in place of the shell

### Requirement: Service-dependent content lazy-loads with a delayed skeleton

Content that depends on a service SHALL load lazily. A skeleton placeholder SHALL
appear only when the content is still pending after a short grace delay, so that
content resolving quickly never flashes a skeleton; once available, the content
SHALL replace any skeleton.

#### Scenario: No skeleton flash on fast resolve

- **WHEN** content depends on a service and resolves within the grace delay
- **THEN** the content is shown directly without ever displaying a skeleton

#### Scenario: Skeleton after the grace delay while slow

- **WHEN** content depends on a service and is still pending after the grace delay
- **THEN** a skeleton placeholder is shown in its place

#### Scenario: Content replaces skeleton when ready

- **WHEN** the service produces the content
- **THEN** the skeleton is replaced by the content

#### Scenario: Available content is shown immediately

- **WHEN** some content is already available while other parts are still pending
- **THEN** the available content is shown
- **AND** only the parts still pending past the grace delay show skeletons

#### Scenario: Content backed by an already-open source does not show a skeleton

- **WHEN** content reads from a source that is already available at shell render
- **THEN** it renders directly without a skeleton, even if other service-dependent content is still pending

### Requirement: Failure degrades to the indicator, not a wall

A service failure during boot SHALL degrade to the read-only health indicator and
SHALL NOT present a blocking modal or full-screen error in place of the shell.

#### Scenario: Failure shows indicator, shell stays usable

- **WHEN** a service fails during boot
- **THEN** the failure is reflected in its health indicator
- **AND** the application shell remains rendered and usable

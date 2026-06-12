# Surface Runtime

## ADDED Requirements

### Requirement: Surface creation dispatches by kind

The surface runtime SHALL bring a surface to life only through `launch_surface`, which dispatches by
the surface's kind. In 0.x the only runnable kind is `terminal`; a `terminal` surface SHALL spawn
its command through the generic spawn and yield the per-surface proxy the runtime owns. A kind with
no launch adapter (e.g. `diff`) SHALL fail with a typed unsupported-kind error and create no proxy.

#### Scenario: Terminal kind spawns and yields a proxy
- **WHEN** a terminal surface is created
- **THEN** the generic spawn runs the command and returns the proxy the runtime stores

#### Scenario: An unsupported kind fails loudly
- **WHEN** a surface of a kind with no launch adapter (e.g. `diff`) is created
- **THEN** the runtime returns a typed unsupported-kind error and stores no proxy

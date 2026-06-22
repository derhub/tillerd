## ADDED Requirements

### Requirement: The domain-model module holds only the domain model

The module that defines the domain model SHALL contain only aggregates and entities, value objects, identifiers, and enums. It SHALL NOT contain input DTOs whose sole purpose is to carry the arguments of a create (or other) operation. An input DTO is identified by its role — it names the parameters of an operation, not a thing the system persists and reasons about.

#### Scenario: A create-input DTO does not live in the domain-model module

- **WHEN** a type exists only to carry the arguments for creating an entity
- **THEN** it is defined in the layer that consumes it, not alongside the domain entities

#### Scenario: Aggregates, value objects, ids, and enums remain in the domain-model module

- **WHEN** a type is an aggregate, a value object, an identifier, or a domain enum
- **THEN** it remains in the domain-model module

### Requirement: A command holds its inputs directly, with no wrapper struct

A create command consumed through a single dispatch SHALL carry its input fields directly on the command type. An input struct whose only consumer is the one command that destructures it SHALL NOT exist — the command is the input message, and a separate "parameters" struct is needless indirection.

#### Scenario: A command-only input is inlined into the command

- **WHEN** an input struct is referenced only by one command and its helpers
- **THEN** its fields are defined directly on the command type and the wrapper struct does not exist

#### Scenario: A command does not nest a parameters struct

- **WHEN** a create command needs several input fields
- **THEN** the fields are members of the command struct, not members of a nested params/draft value it holds

### Requirement: The command handler translates input into a domain entity

A create command's handler SHALL be the single point that turns the input into a fully-formed domain entity: it validates the input, applies defaults, constructs value objects, and sets the caller-assigned identifier. Defaults and identifier minting SHALL NOT occur in the persistence layer.

#### Scenario: Defaults and value objects are built in the handler

- **WHEN** a create command is handled
- **THEN** the handler produces the complete entity — identifier, defaults, and value objects all set — before any persistence call

#### Scenario: The persistence layer applies no defaults

- **WHEN** an entity is handed to a repository to persist
- **THEN** the repository writes the entity's fields as given and supplies no default or generated value of its own

### Requirement: Repositories accept domain entities only

A repository's create operation SHALL accept a domain entity and persist it. The persistence layer SHALL NOT reference any input, draft, or command type — it knows only entities and value objects. A repository create SHALL return no data, because the caller already holds the entity it supplied.

#### Scenario: Create takes the entity, not an input DTO

- **WHEN** a repository persists a newly created aggregate
- **THEN** its create operation receives the entity itself and the persistence module imports no input/draft/command type

#### Scenario: No input type survives the boundary

- **WHEN** the create path is followed from command to repository
- **THEN** the input type exists only in the application layer and no equivalent draft exists in the domain-model or persistence layers

### Requirement: A deleted input type leaves no name collision

Removing the input/draft types SHALL leave each create operation named by a single unambiguous type. Where a draft previously shared a name with its command, the draft's deletion SHALL remove the collision with no remaining import alias.

#### Scenario: The command name is unique after the draft is deleted

- **WHEN** a draft that shared a name with its command is deleted
- **THEN** the command is the only type with that name and no import alias remains

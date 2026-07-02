# Capability: generated-ipc-bindings

## Purpose

TBD — Auto-generation of TypeScript bindings from Rust IPC shims.

## Requirements

### Requirement: Bindings are generated from the Rust IPC layer

The TypeScript bindings for the desktop IPC surface SHALL be generated from the Rust command and event definitions, not hand-written, and SHALL live in a dedicated package (`@tillerd/client-bindings`) that carries the Tauri runtime dependency. The generator SHALL cover every registered IPC command and query, and the emitted artifact SHALL be the single source of truth for the desktop wire types and the typed invoke client. Adding a new command or changing an argument or return type in Rust SHALL be the only edit required for the binding to change.

#### Scenario: A registered command appears in the bindings

- **WHEN** an IPC command is registered in the transport handler list
- **THEN** the generated bindings expose a typed wrapper for that command with its argument and return types
- **AND** no hand-written TypeScript declares that command's types

#### Scenario: A Rust type change flows to TypeScript

- **WHEN** a wire DTO or argument type changes in Rust
- **THEN** regenerating the bindings reflects the change in the TypeScript types
- **AND** no separate hand edit to the TypeScript types is required

### Requirement: Wire types are emitted separately from the Tauri client

The generated output SHALL be split into a pure types module and a client module. The types module SHALL contain only TypeScript type declarations and SHALL import nothing (in particular, no Tauri runtime). The client module (commands + events) SHALL import its types from the types module and MAY carry the Tauri runtime dependency. A consumer that needs only wire shapes SHALL be able to import the types module without pulling in the Tauri client.

#### Scenario: The types module is import-free

- **WHEN** the bindings are generated
- **THEN** the types module declares the wire types with no import statements
- **AND** the client module imports those types from the types module

#### Scenario: A consumer imports types without the client

- **WHEN** a caller imports only the wire types
- **THEN** no Tauri runtime code is pulled into that caller

### Requirement: The pure SDK stays free of the Tauri dependency

Generation SHALL NOT add the Tauri runtime dependency to `@tillerd/sdk`. The Tauri-coupled generated client SHALL reside only in `@tillerd/client-bindings`, so `@tillerd/sdk` remains zero-runtime-dependency and transport-agnostic for its other consumers.

#### Scenario: The SDK core keeps zero runtime dependencies

- **WHEN** the bindings are generated
- **THEN** `@tillerd/sdk` declares no Tauri runtime dependency
- **AND** the generated Tauri client is imported only from `@tillerd/client-bindings`

### Requirement: Generated events are typed

Outbound events emitted by the backend SHALL be represented in the generated bindings with typed payloads, so subscribers receive a typed event rather than an untyped payload.

#### Scenario: An emitted event is typed at the subscriber

- **WHEN** a caller subscribes to a backend event through the generated bindings
- **THEN** the event payload is delivered with its generated type

### Requirement: Wire shape is preserved

Generation SHALL NOT change the wire contract. Command names, argument shapes, response JSON, and error strings SHALL remain byte-identical to the contract asserted by the desktop command-contract test.

#### Scenario: The command contract still passes

- **WHEN** the bindings are generated and the command-contract test runs
- **THEN** every command's wire shape matches the asserted contract unchanged

### Requirement: Bindings cannot drift

The generated bindings SHALL be committed to the repository, and a test SHALL regenerate them and fail if the result differs from the committed file. A change to the Rust IPC surface that is not accompanied by regenerated bindings SHALL fail that test.

#### Scenario: Stale bindings fail the build

- **WHEN** the Rust IPC surface changes but the committed bindings are not regenerated
- **THEN** the drift-guard test fails
- **AND** regenerating the bindings makes it pass

### Requirement: Renderer IPC goes through the generated bindings only

Renderer code SHALL invoke desktop IPC exclusively through the generated bindings
(`@tillerd/client-bindings`). Raw string-keyed `invoke` calls are permitted only in the generated
bindings file itself and in the transport core loader that constructs the invoke capability. The
constraint SHALL be enforced by a structural rule that fails CI at `error` severity.

#### Scenario: A raw invoke outside the sanctioned homes fails CI

- **WHEN** renderer or binding code calls `invoke("<command>", ...)` outside
  `tauri_bindings.gen.ts` or the transport core loader
- **THEN** the structural rule reports an error-severity finding and the arch gate fails

#### Scenario: A new command is consumed via its generated binding

- **WHEN** a new desktop command is added to the specta export
- **THEN** the renderer reaches it through the regenerated typed binding, never a hand-written
  string invoke

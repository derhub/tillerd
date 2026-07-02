## ADDED Requirements

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

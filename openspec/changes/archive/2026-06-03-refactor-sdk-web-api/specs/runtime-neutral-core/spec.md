## ADDED Requirements

### Requirement: Contract and engine layers run in a standard web runtime

The contract layer and the engine layer SHALL depend only on standard web runtime APIs, so they
load and run unchanged in a host that provides no server-runtime-only globals.

#### Scenario: Loaded in a web runtime host

- **WHEN** the contract and engine layers are loaded in a host whose globals are limited to
  standard web runtime APIs (standard byte arrays, text encoders/decoders, the standard crypto
  and data-view APIs)
- **THEN** they operate without referencing any server-runtime-only global (byte-buffer objects,
  process/environment objects, or a module-require)

### Requirement: Wire framing and snapshot rendering use standard byte APIs

Frame encode/decode and snapshot-to-bytes SHALL operate on standard byte arrays and text
encoders, producing byte-identical output to the previous server-runtime implementation.

#### Scenario: Encoding and decoding a frame

- **WHEN** a frame with or without a binary body is encoded and then decoded in any host
- **THEN** the byte output is identical to the previous implementation
- **AND** the decoded binary body is exposed as a standard byte array

#### Scenario: Rendering a snapshot to bytes

- **WHEN** a screen snapshot is rendered to bytes
- **THEN** the result is a standard byte array identical to the previous output

### Requirement: Runtime neutrality is enforced

The verification step SHALL fail if the contract or engine source tree references a
server-runtime-only global or module loader.

#### Scenario: A server-runtime global re-enters a neutral layer

- **WHEN** the verification step runs and either source tree references a byte-buffer object,
  process/environment object, a module-require, or a server-runtime namespace
- **THEN** verification fails

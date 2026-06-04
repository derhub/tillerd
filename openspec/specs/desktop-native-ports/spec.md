# desktop-native-ports

## Purpose

The native (Tauri) implementations of the engine's injected ports on the desktop path: a daemon
byte bridge (raw, ordered, back-pressured), file-read for transcript reads, and a logger.

## Requirements

### Requirement: Native daemon-transport implementation for the in-view engine

The native core SHALL provide the engine's daemon-transport contract to the renderer by
bridging the renderer and the daemon: forwarding the renderer's outbound bytes to the daemon's
local channel and delivering the daemon's output bytes back to the renderer. The native core
SHALL forward bytes verbatim and SHALL NOT interpret the wire framing.

#### Scenario: Outbound bytes reach the daemon

- **WHEN** the renderer sends encoded protocol bytes through the transport
- **THEN** the native core forwards exactly those bytes to the daemon's local channel

#### Scenario: Daemon output reaches the renderer

- **WHEN** the daemon produces output bytes for a session
- **THEN** the native core delivers exactly those bytes to the renderer, byte-for-byte

### Requirement: Ordered, raw byte delivery

The native transport SHALL deliver each session's output to the renderer as an ordered stream
of raw bytes, without ANSI stripping, character re-decoding, or numeric re-encoding.

#### Scenario: Output ordering and fidelity are preserved

- **WHEN** the daemon produces output in a given order
- **THEN** the renderer receives that output in the same order, byte-for-byte

### Requirement: Backpressure preserved across the bridge

The native transport SHALL preserve the daemon's flow-control loop so that a fast-producing
session cannot overwhelm a slower renderer, returning consumption credit as the renderer drains
output, with no bytes dropped or reordered.

#### Scenario: Slow renderer applies backpressure

- **WHEN** a session produces output faster than the renderer consumes it
- **THEN** the transport withholds further output until the renderer returns consumption credit
- **AND** no output bytes are dropped or reordered

### Requirement: Native file-read implementation for transcript reads

The native core SHALL provide the engine's file-read contract, returning the size of a
transcript file and the bytes of a requested range, and reporting an absent file distinctly.

#### Scenario: Reading a transcript delta

- **WHEN** the renderer requests the current size and then a byte range of a transcript file
- **THEN** the native core returns the size and the requested bytes

#### Scenario: Absent transcript file

- **WHEN** the renderer requests the size of a transcript file that does not exist
- **THEN** the native core reports the file as absent rather than returning a size

### Requirement: Logger implementation for the in-view engine

The native host SHALL provide a logger implementation satisfying the engine's logger contract,
so engine diagnostics are captured rather than discarded.

#### Scenario: Engine diagnostics are captured

- **WHEN** the in-view engine emits a diagnostic
- **THEN** the supplied logger records it through the native host's diagnostic channel

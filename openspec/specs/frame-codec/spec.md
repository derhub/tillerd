# frame-codec

## Purpose

Defines the canonical length-prefixed frame codec for loopback IPC streams: one implementation per language runtime, wire-compatible across runtimes, with a bounded maximum frame size, incremental decoding across partial reads, and payload-agnostic framing.

## Requirements

### Requirement: Length-prefixed frame format

A frame on a loopback IPC stream SHALL consist of a 4-byte big-endian unsigned payload length followed by exactly that many payload bytes. Encoding a payload SHALL produce this layout, and decoding SHALL recover the original payload bytes unchanged.

#### Scenario: Encode then decode round-trips the payload

- **WHEN** a payload is encoded into a frame and that frame is decoded
- **THEN** the decoded payload bytes equal the original payload bytes

#### Scenario: Multiple frames in one stream decode in order

- **WHEN** two frames are encoded back to back into a single byte stream and decoded
- **THEN** the decoder yields the two payloads in the order they were written

### Requirement: One canonical frame codec per language runtime

Within a single language runtime, every loopback IPC face and every consumer of those faces SHALL obtain its length-prefix framing from one canonical source rather than re-deriving its own implementation. A capability that carries a different wire (for example, one with a separate raw body plane) is out of scope and retains its own codec.

#### Scenario: Same-runtime faces and consumers share one implementation

- **WHEN** a producing face and a consuming client in the same language runtime exchange frames
- **THEN** both obtain the encode and decode behavior from the single canonical source for that runtime, not from per-component copies

### Requirement: Cross-language codecs stay wire-compatible

When the same logical wire is spoken by more than one language runtime, each runtime SHALL keep its own canonical codec, and those codecs SHALL produce and accept byte-identical frames. Unifying them into a single implementation is not required and not possible across the language boundary; byte compatibility is the contract that binds them.

#### Scenario: A frame encoded in one runtime decodes in another

- **WHEN** a frame is encoded by one runtime's canonical codec and read by another runtime's canonical codec
- **THEN** the second runtime recovers the original payload bytes unchanged

### Requirement: Bounded frame size on every decode path

The codec SHALL enforce a fixed maximum frame size. When a decode path reads a length prefix that declares a payload larger than the maximum, it SHALL reject the frame before allocating buffer space for the declared length. This bound SHALL apply identically on every decode path, including incremental decoding driven by a consumer.

#### Scenario: Oversize length prefix is rejected before allocation

- **WHEN** a decode path reads a length prefix declaring a payload larger than the maximum frame size
- **THEN** it reports an invalid-data error and does not allocate a buffer for the declared length

#### Scenario: Consumer-driven decoding enforces the same bound

- **WHEN** a consumer feeds bytes whose length prefix exceeds the maximum frame size to the incremental decoder
- **THEN** the decoder rejects the frame rather than buffering toward the declared length

### Requirement: Incremental decoding across partial reads

The codec SHALL provide a decoder that accepts arbitrary byte chunks and returns every complete frame currently available, retaining any partial frame across feeds so that a frame split across multiple chunks is recovered once its bytes have all arrived.

#### Scenario: Frame split across chunks is recovered

- **WHEN** a single frame's bytes are delivered in two separate chunks
- **THEN** the decoder returns no frame after the first chunk and the complete frame after the second

#### Scenario: Clean end of stream yields no partial frame

- **WHEN** a stream ends exactly on a frame boundary
- **THEN** the decoder reports no further frames and no error

### Requirement: Payload-agnostic framing

The frame codec SHALL operate on opaque payload bytes and SHALL NOT interpret, validate, or depend on the structure of the payload. The choice of payload encoding belongs to the caller, so the codec can frame any payload format without change.

#### Scenario: Codec frames arbitrary payload bytes

- **WHEN** a caller encodes a payload whose contents the codec does not interpret
- **THEN** the frame is produced and later decoded without the codec inspecting or transforming the payload bytes

## Context

The loopback IPC faces and their consumers share a wire shape — a 4-byte big-endian length prefix followed by a payload — but each component carries its own copy of the encode/decode code. Four Rust copies are byte-identical (a producing face, a consumer client, a second consumer in the gateway, and a synchronous admin client in the desktop app). Two further Rust copies belong to a different wire (the pseudo-terminal stream, which adds a separate raw body plane) and are deliberately out of scope. A parallel set of copies exists in the TypeScript runtime, which cannot import a Rust crate and so is consolidated separately.

The shared-wire-version constants already live in one contracts crate that every affected component depends on. The framing logic — which is what those versions version — does not. That split is the source of the duplication: there is no obvious home for the codec, so each component grew its own.

One of the three identical copies (the consumer client) omits the maximum-frame-size bound the other two enforce. A hostile or corrupted length prefix can therefore drive that consumer to buffer toward an arbitrarily large declared length.

## Goals / Non-Goals

**Goals:**

- One canonical frame codec, owned by the contracts crate next to the wire versions it serves.
- Every JSON-payload loopback face and consumer uses that one codec.
- The maximum-frame-size bound applies on every decode path, closing the consumer-side gap.
- No change to the bytes on the wire and no wire-version bump.

**Non-Goals:**

- No transport abstraction or `Transport` trait — there is no second transport requirement to design against yet.
- No payload-format change — payloads stay as they are; the codec remains payload-agnostic.
- No in-process/embedding change.
- The pseudo-terminal wire (length prefix + meta + body separator + raw body plane) keeps its own codec; it is a different wire on the hot path.
- No TypeScript-side consolidation — the TS canonical home (the shared SDK) stays byte-compatible but is deduplicated in a separate follow-up change.

## Decisions

### Move only the pure pieces into the contracts crate; keep the async I/O adapters at the face

The contracts crate is intentionally dependency-light (serialization plus a compile-time error derive, no async runtime). The shared codec splits cleanly into a pure core and runtime-bound adapters:

- **Pure core -> contracts crate:** `encode_frame(bytes) -> bytes`, the `MAX_FRAME_SIZE` constant, and an incremental push-based decoder that takes byte chunks and returns complete frames or a typed oversize error. These need only the standard library and a compile-time error derive.
- **Async adapters -> stay at the producing face:** the `read_frame`/`write_frame` helpers that drive an async stream are thin wrappers over the pure `encode_frame` and the size bound. They keep their runtime dependency local to the face that already has it.

Alternative considered — move the async helpers into the contracts crate too: rejected because it would add an async-runtime dependency to a crate every component links, including ones that have no async I/O. The pure/adapter split keeps the contracts crate runtime-free, which is also the guardrail that preserves a future transport swap (the framing stays both transport-agnostic and payload-agnostic).

Alternative considered — a brand-new dedicated framing crate: rejected as unnecessary indirection. The framing versions the same wire whose version constants already live in the contracts crate; co-locating them is the simpler home and avoids a second crate that every component would have to depend on anyway.

### The shared decoder carries the size bound

The incremental decoder applies `MAX_FRAME_SIZE` when it reads a length prefix, rejecting before it buffers toward the declared length. Because the consumer client switches to this shared decoder, it inherits the bound it currently lacks. For a valid frame the behavior is identical; only an oversize declaration changes outcome (now rejected instead of buffered).

### The pseudo-terminal wire stays separate

That wire carries a meta header plus an optional raw body plane, not a single opaque payload. Folding it into the shared codec would force the shared core to model a body plane it does not need and would entangle the hot path with the control-plane codec. It keeps its own implementation.

### The synchronous desktop client shares the pure core, not the async adapters

The desktop admin client drives a blocking socket, so it cannot use the async `read_frame`/`write_frame` adapters. The pure/adapter split makes this a non-issue: it imports `encode_frame` and `MAX_FRAME_SIZE` (both runtime-free) and keeps its own small blocking read loop. It stops re-deriving the encode and the bound while staying synchronous. Including it requires adding a dependency on the contracts crate to the desktop app — acceptable because that crate is pure and dependency-light, and the app already speaks this exact wire.

### One canonical codec per runtime, not one codec everywhere

The contracts crate is a Rust crate; the TypeScript runtime cannot import it. The same logical wire is therefore spoken by two runtimes, and the honest contract is **one canonical codec per runtime, kept byte-compatible**, not a single shared implementation. This change establishes the Rust canonical home (the contracts crate). The TypeScript canonical home (the shared SDK) already exists and stays byte-compatible; its own internal duplication (a server module that re-derives the codec instead of importing the SDK) is a separate follow-up. Pretending one implementation could serve both runtimes would be a false claim the spec must not make — hence the explicit cross-language wire-compatibility requirement instead.

## Risks / Trade-offs

- **A consumer relied on accepting oversize frames** -> No known consumer sends frames above the existing cap; the producing faces already enforce it, so an oversize frame could not be produced on these paths today. The change only makes the consumer reject what no producer emits.
- **Behavioral drift between the moved code and the originals** -> Move the existing tests (round-trip, oversize rejection, clean end-of-stream) into the contracts crate alongside the codec, and keep a face↔consumer round-trip assertion, so the shared implementation is pinned by the same cases that pinned the copies.
- **A future caller assumes the codec understands its payload** -> The codec is specified as payload-agnostic (opaque bytes in, opaque bytes out). Payload validation stays with the caller, where the trust-boundary checks already live.

## Migration Plan

1. Add the `framing` module (pure core + tests) to the contracts crate.
2. Switch the consumer client to import the shared core; delete its local copy; keep its subscribe-domain types.
3. Switch the producing face to the shared `encode_frame`/`MAX_FRAME_SIZE`; keep its async adapters as thin wrappers; delete its local constants.
4. Switch the gateway consumer to import the shared core; delete its copy.
5. Add a contracts-crate dependency to the desktop app; switch its synchronous admin client to the shared `encode_frame` + `MAX_FRAME_SIZE`; keep its blocking read loop.
6. Run the full workspace test suite plus the face↔consumer round-trip.

Rollback is a straight revert: the on-wire bytes never changed, so a reverted build interoperates with a non-reverted peer.

## Open Questions

None.

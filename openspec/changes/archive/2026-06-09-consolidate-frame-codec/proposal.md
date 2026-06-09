## Why

The length-prefixed JSON frame codec (4-byte big-endian length, then payload) is hand-written in four byte-identical Rust copies — `apps/gate/src/endpoint/mod.rs`, `packages/gate-client/src/lib.rs`, `apps/mcp-gateway-rs/src/gate_ipc.rs`, and `apps/desktop/src-tauri/src/gate_admin.rs`. Each copy is a separate place the wire can silently drift, yet `contracts-rs` already owns the wire-version constants these codecs version against. One copy (`gate-client`) also lacks the size cap the others enforce, leaving a latent unbounded-allocation path on a hostile length prefix.

## What Changes

- Add a pure `framing` module to `contracts-rs`: `encode_frame`, `MAX_FRAME_SIZE`, and an incremental `FrameDecoder` (+ `RawFrame`) that applies the size bound. No runtime/transport dependency — `contracts-rs` stays `tokio`-free; it adds only `thiserror` (a compile-time derive, already in the workspace build) for the typed decode error.
- `packages/gate-client`: remove its local `encode_frame` / `FrameDecoder` / `RawFrame` / `HEADER_SIZE`; import them from `contracts-rs`. Keep the subscribe-domain types (`SubscriptionFrame`, `decode_subscription_frame`, `negotiate_ready`).
- `apps/gate/src/endpoint/mod.rs`: remove its local `encode_frame` and size constants; keep the async `read_frame`/`write_frame` as thin `tokio` adapters built on the shared `encode_frame` + `MAX_FRAME_SIZE`.
- `apps/mcp-gateway-rs/src/gate_ipc.rs`: remove its copy; import from `contracts-rs`.
- `apps/desktop/src-tauri/src/gate_admin.rs`: add a `contracts-rs` dependency to the desktop app; share `encode_frame` + `MAX_FRAME_SIZE`. This client is synchronous (blocking sockets), so it keeps its own small sync read loop but stops re-deriving the encode and the bound.
- Hardening (not a behavior change for valid frames): the shared `FrameDecoder` carries `MAX_FRAME_SIZE`, so `gate-client` and any future consumer gain the cap it currently lacks.

Not breaking: the on-wire bytes are unchanged and no wire-version is bumped.

## Capabilities

### New Capabilities

- `frame-codec`: the shared length-prefixed JSON frame codec owned by `contracts-rs` — encode, incremental decode, the size bound, and the rule that all loopback IPC faces use this single implementation rather than re-deriving it.

### Modified Capabilities

<!-- None. Framing is not currently spec-governed; consumer behavioral requirements
     (forward payload, fire-and-forget, auth, route-by-session) are unchanged. The
     on-wire bytes are identical, so no existing capability's requirements change. -->

## Impact

- **Gains code:** `packages/contracts-rs` (new `framing` module + its tests).
- **Loses duplication:** `packages/gate-client`, `apps/gate`, `apps/mcp-gateway-rs`, `apps/desktop` (`gate_admin.rs`).
- **Untouched:** `packages/daemon-pty` and `packages/daemon-pty-client` keep their own codec — a different wire (JSON meta + `0x0a` separator + raw body plane) on the hot path. Not shared.
- **Dependencies:** `gate-client`, `gate`, and `mcp-gateway-rs` already depend on `contracts-rs` — no new edge. The desktop app gains a `contracts-rs` dependency (a pure, dependency-light crate). `contracts-rs` gains `thiserror` (a compile-time derive already in the workspace build) for the decode error and stays `tokio`-free.
- **Test mock left as-is:** `apps/desktop/src-tauri/src/orchestrator.rs` has an inline length-prefix copy inside `#[cfg(test)] mod tests` (a fake admin socket). Test fixtures may be self-contained, so it is not folded in.

### Cross-language framing (not unified here)

`contracts-rs` is a Rust crate; the TypeScript side cannot import it. A parallel framing implementation therefore exists on the TS side and is **out of scope** for this change:

- `@athing/sdk` is the TS canonical home — `protocol/codec.ts` (the pseudo-terminal wire) and `types/subscription.ts` (the gate JSON wire).
- `apps/server/src/gate-admin.ts` re-derives its own `encodeFrame` instead of importing the sdk codec — a TS-side duplication that mirrors the Rust `gate_admin.rs` one.

This change does not unify TS and Rust into one codec — that is impossible across the language boundary. It establishes the Rust canonical home (`contracts-rs`); the TS canonical home (`@athing/sdk`) stays byte-compatible with it. Deduping `apps/server/gate-admin.ts` into `@athing/sdk` is a **separate follow-up TS change**, tracked but not done here.

### Out of scope

- No transport abstraction / `Transport` trait.
- No payload-format change (JSON stays).
- No in-process/embedding change.
- No `daemon-pty` wire change.
- No TS-side consolidation (separate follow-up).

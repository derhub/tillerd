## 1. Add the shared codec to the contracts crate

- [x] 1.1 Add a `framing` module to `packages/contracts-rs` exposing `encode_frame(&[u8]) -> Vec<u8>`, the `MAX_FRAME_SIZE` constant, `RawFrame`, and a push-based `FrameDecoder`.
- [x] 1.2 Apply the `MAX_FRAME_SIZE` bound inside `FrameDecoder` so an oversize length prefix is rejected before buffering toward the declared length.
- [x] 1.3 Move the codec tests (encode/decode round-trip, multiple-frames-in-order, oversize rejection, frame-split-across-chunks, clean end-of-stream) into the `framing` module.
- [x] 1.4 Confirm `packages/contracts-rs/Cargo.toml` gains no new dependency (no async runtime); `cargo test -p contracts-rs` passes.

## 2. Switch the consumer client to the shared codec

- [x] 2.1 In `packages/gate-client`, remove the local `encode_frame`, `FrameDecoder`, `RawFrame`, and `HEADER_SIZE`; import them from the contracts crate.
- [x] 2.2 Keep the subscribe-domain types (`SubscriptionFrame`, `decode_subscription_frame`, `negotiate_ready`) in `gate-client`.
- [x] 2.3 `cargo test -p gate-client` passes; the consumer now rejects oversize frames.

## 3. Switch the producing face to the shared codec

- [x] 3.1 In `apps/gate/src/endpoint/mod.rs`, remove the local `encode_frame`, `HEADER_SIZE`, and `MAX_FRAME_SIZE`; import the shared `encode_frame` and `MAX_FRAME_SIZE`.
- [x] 3.2 Keep `read_frame`/`write_frame` as thin async adapters built on the shared `encode_frame` and bound.
- [x] 3.3 Update the D9 comment to point at the shared codec instead of describing a local reimplementation.
- [x] 3.4 `cargo test -p gate` (or the gate package) passes.

## 4. Switch the gateway consumer to the shared codec

- [x] 4.1 In `apps/mcp-gateway-rs/src/gate_ipc.rs`, remove the local length-prefix codec; import it from the contracts crate.
- [x] 4.2 `cargo test -p mcp-gateway-rs` (or the gateway package) passes.

## 5. Switch the desktop admin client to the shared codec

- [x] 5.1 Add a `contracts-rs` dependency to `apps/desktop/src-tauri/Cargo.toml`.
- [x] 5.2 In `apps/desktop/src-tauri/src/gate_admin.rs`, replace the local `write_frame` encode and `MAX_FRAME` constant with the shared `encode_frame` + `MAX_FRAME_SIZE`; keep the synchronous blocking read loop.
- [x] 5.3 Leave the inline length-prefix copy in `orchestrator.rs` `#[cfg(test)] mod tests` (`fake_admin_socket`) as-is — a self-contained test mock, out of scope.
- [x] 5.4 `cargo test` for the desktop crate passes.

## 6. Verify end to end

- [x] 6.1 Run the full workspace test suite (`cargo test` across all crates).
- [x] 6.2 Confirm a producing-face ↔ consumer-client frame round-trip still passes.
- [x] 6.3 Confirm `packages/daemon-pty` and `packages/daemon-pty-client` are unchanged (their wire is out of scope).
- [x] 6.4 Confirm no TypeScript files changed — the TS canonical home (`@athing/sdk`) and the `apps/server/gate-admin.ts` duplication are a separate follow-up, not part of this change.

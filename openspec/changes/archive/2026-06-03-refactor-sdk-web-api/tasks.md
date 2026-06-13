## 1. sdk wire codec -> standard byte APIs

- [x] 1.1 Rewrite `packages/sdk/src/protocol/codec.ts` `encodeFrame` using `Uint8Array` +
      `DataView` (4-byte big-endian length) + `TextEncoder`; concatenate header/meta/body as
      `Uint8Array`. Return `Uint8Array` instead of `Buffer`.
- [x] 1.2 Rewrite `FrameDecoder` to accumulate `Uint8Array` (no `Buffer.concat`), read the length
      via `DataView.getUint32`, split meta/body on the `0x0a` separator via `indexOf`; expose
      `DecodedFrame.body` as `Uint8Array | null`.
- [x] 1.3 Add sdk codec tests asserting `Uint8Array` bodies, round-trip, multi-frame and
      split-chunk framing, and deterministic byte output (`packages/sdk/tests/codec.test.ts`).

## 2. sdk snapshot renderer + signals -> neutral

- [x] 2.1 `packages/sdk/src/protocol/snapshot-render.ts`: replace `Buffer.from(...)` with
      `new TextEncoder().encode(...)`; return a real `Uint8Array`.
- [x] 2.2 `packages/sdk/src/signals.ts`: remove the `process.platform` reference (guard or
      parameterize); no server-runtime global in sdk.

## 3. Ripple to Bun consumers

- [x] 3.1 `packages/platform-bun/src/daemon-transport.ts`: drop the `Buffer`-typed write cast;
      pass the `Uint8Array` frame to `socket.write` directly.
- [x] 3.2 Confirm the daemon (its own `protocol/codec.ts`) is untouched and still green.

## 4. Enforce + verify

- [x] 4.1 Add a grep gate over `packages/sdk/src` AND `packages/engine/src`: no `Buffer`,
      `node:`, `Bun.`, `process.`, or `require(`. Wire it into the check step.
- [x] 4.2 `bun test` green across sdk, engine, platform-bun, server, integration.
- [x] 4.3 `turbo run check-types` clean.
- [x] 4.4 Grep gate passes for both sdk and engine.

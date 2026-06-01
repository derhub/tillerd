# 0009. Binary framing protocol for daemon IPC

- Status: accepted
- Date: 2026-06-02

## Context

The original daemon wire format was newline-delimited JSON (NDJSON). PTY output was encoded as base64 or JSON-escaped strings inside the JSON envelope. At 3–5 concurrent sessions this approach has two structural problems: ~33% bandwidth overhead for binary PTY data, and no way to embed raw bytes without costly re-encoding.

The engine client was coupled to NDJSON via a readline parser, leaving no room for binary payloads or version-independent protocol evolution.

## Decision

Replace NDJSON framing with a length-prefixed binary frame format:

```
┌──────────────────────────┐
│ 4-byte big-endian uint32 │  total payload length
├──────────────────────────┤
│ UTF-8 JSON metadata      │  always present
├──────────────────────────┤
│ 0x0A newline separator   │  present only when binary body follows
├──────────────────────────┤
│ raw binary body          │  optional; carries PTY output bytes
└──────────────────────────┘
```

This format is implemented in `packages/daemon/src/protocol/codec.ts` as `encodeFrame` / `FrameDecoder`.

A versioned `hello`/`hello-ack` handshake (D5) gates the connection. The `hello` frame carries an array of integer version numbers the client supports; the daemon picks the highest mutually supported version. This allows independent evolution of client and daemon binaries during rolling upgrades.

## Consequences

- PTY output (`data` frames) carries raw bytes in the binary body — no base64 overhead.
- `FrameDecoder` is stateful and streaming: it handles arbitrary chunk boundaries without buffering an entire message into a string.
- The codec is used on both the client→daemon wire and the IPC channel between predecessor and successor during upgrade.
- A client that fails version negotiation receives an `error { code: "EVERSION" }` frame and the connection is closed immediately.
- The NDJSON encoder (`ndjson.ts`) and old protocol module (`protocol.ts`) are removed; all consumers migrate to the new codec.

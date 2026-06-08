import type { HookEvent } from "./events";

export const HOOK_SUBSCRIPTION_WIRE_VERSION = 1;

export interface HookSubscribeRequest {
  sessionId: string;
  wireVersion: number;
}

export class RawFrame {
  constructor(readonly payload: Uint8Array) {}
}

const HEADER_SIZE = 4;

export function encodeFrame(payload: Uint8Array): Uint8Array {
  const out = new Uint8Array(HEADER_SIZE + payload.length);
  const view = new DataView(out.buffer);
  view.setUint32(0, payload.length, false);
  out.set(payload, HEADER_SIZE);
  return out;
}

export function encodeSubscribeRequest(request: HookSubscribeRequest): Uint8Array {
  const json = new TextEncoder().encode(JSON.stringify(request));
  return encodeFrame(json);
}

export class FrameDecoder {
  private buf: Uint8Array = new Uint8Array(0);

  push(chunk: Uint8Array): RawFrame[] {
    const merged = new Uint8Array(this.buf.length + chunk.length);
    merged.set(this.buf, 0);
    merged.set(chunk, this.buf.length);
    this.buf = merged;

    const results: RawFrame[] = [];
    let offset = 0;

    while (this.buf.length - offset >= HEADER_SIZE) {
      const view = new DataView(this.buf.buffer, this.buf.byteOffset + offset);
      const payloadLen = view.getUint32(0, false);
      if (this.buf.length - offset < HEADER_SIZE + payloadLen) break;
      const start = offset + HEADER_SIZE;
      results.push(new RawFrame(this.buf.slice(start, start + payloadLen)));
      offset = start + payloadLen;
    }

    this.buf = offset > 0 ? this.buf.slice(offset) : this.buf;
    return results;
  }
}

export type SubscriptionFrame =
  | { type: "Ready"; wireVersion: number }
  | { type: "Event"; event: HookEvent }
  | { type: "Error"; reason: string }
  | { type: "Other"; frame: string };

export type DecodeError =
  | { kind: "WireVersionMismatch"; expected: number; got: number }
  | { kind: "Rejected"; reason: string }
  | { kind: "UnexpectedFrame" };

export function decodeSubscriptionFrame(frame: RawFrame): SubscriptionFrame | null {
  let meta: Record<string, unknown>;
  try {
    meta = JSON.parse(new TextDecoder().decode(frame.payload)) as Record<string, unknown>;
  } catch {
    return null;
  }

  const frameKind = meta["frame"];
  if (typeof frameKind !== "string") return null;

  switch (frameKind) {
    case "ready": {
      const wv = meta["wireVersion"];
      return {
        type: "Ready",
        wireVersion: typeof wv === "number" ? wv : 0,
      };
    }
    case "event": {
      const ev = meta["event"];
      if (!ev || typeof ev !== "object") return null;
      return { type: "Event", event: ev as HookEvent };
    }
    case "error": {
      const reason = meta["reason"];
      return { type: "Error", reason: typeof reason === "string" ? reason : "" };
    }
    default:
      return { type: "Other", frame: frameKind };
  }
}

export function negotiateReady(
  frame: SubscriptionFrame,
): { ok: true; version: number } | { ok: false; error: DecodeError } {
  if (frame.type === "Ready") {
    if (frame.wireVersion === HOOK_SUBSCRIPTION_WIRE_VERSION) {
      return { ok: true, version: frame.wireVersion };
    }
    return {
      ok: false,
      error: {
        kind: "WireVersionMismatch",
        expected: HOOK_SUBSCRIPTION_WIRE_VERSION,
        got: frame.wireVersion,
      },
    };
  }
  if (frame.type === "Error") {
    return { ok: false, error: { kind: "Rejected", reason: frame.reason } };
  }
  return { ok: false, error: { kind: "UnexpectedFrame" } };
}

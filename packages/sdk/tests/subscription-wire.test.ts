import { test, expect, describe } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import {
  HOOK_SUBSCRIPTION_WIRE_VERSION,
  RawFrame,
  FrameDecoder,
  encodeFrame,
  encodeSubscribePreamble,
  decodeSubscriptionFrame,
  negotiateReady,
} from "../src/types/subscription";

const FIXTURES_DIR = join(import.meta.dir, "../../../crates/gate-client/tests/fixtures");

function rawFromFile(name: string): RawFrame {
  const payload = readFileSync(join(FIXTURES_DIR, name));
  return new RawFrame(new Uint8Array(payload));
}

function rawFromString(json: string): RawFrame {
  return new RawFrame(new TextEncoder().encode(json.trim()));
}

describe("WIRE_VERSION", () => {
  test("equals the contracts constant", () => {
    expect(HOOK_SUBSCRIPTION_WIRE_VERSION).toBe(1);
  });
});

describe("encodeFrame / FrameDecoder round-trip", () => {
  test("single frame round-trips", () => {
    const payload = new TextEncoder().encode("hello");
    const frames = new FrameDecoder().push(encodeFrame(payload));
    expect(frames).toHaveLength(1);
    expect(frames[0]!.payload).toEqual(payload);
  });

  test("decoder holds partial frame across two pushes", () => {
    const encoded = encodeFrame(new TextEncoder().encode('{"frame":"ready","wireVersion":1}'));
    const mid = Math.floor(encoded.length / 2);
    const dec = new FrameDecoder();
    expect(dec.push(encoded.slice(0, mid))).toHaveLength(0);
    expect(dec.push(encoded.slice(mid))).toHaveLength(1);
  });

  test("decoder extracts multiple frames in one push", () => {
    const a = encodeFrame(new TextEncoder().encode("first"));
    const b = encodeFrame(new TextEncoder().encode("second"));
    const combined = new Uint8Array(a.length + b.length);
    combined.set(a, 0);
    combined.set(b, a.length);
    const frames = new FrameDecoder().push(combined);
    expect(frames).toHaveLength(2);
    expect(new TextDecoder().decode(frames[0]!.payload)).toBe("first");
    expect(new TextDecoder().decode(frames[1]!.payload)).toBe("second");
  });

  test("decoder handles payload spanning three chunks", () => {
    const encoded = encodeFrame(new TextEncoder().encode("abcdefghij"));
    const dec = new FrameDecoder();
    dec.push(encoded.slice(0, 2));
    dec.push(encoded.slice(2, 5));
    const frames = dec.push(encoded.slice(5));
    expect(frames).toHaveLength(1);
    expect(new TextDecoder().decode(frames[0]!.payload)).toBe("abcdefghij");
  });
});

describe("encodeSubscribePreamble", () => {
  test("produces a framed subscribe route preamble with sessionId and wireVersion", () => {
    const frames = new FrameDecoder().push(encodeSubscribePreamble("s1"));
    expect(frames).toHaveLength(1);
    const decoded = JSON.parse(new TextDecoder().decode(frames[0]!.payload)) as Record<
      string,
      unknown
    >;
    expect(decoded["route"]).toBe("subscribe");
    expect(decoded["sessionId"]).toBe("s1");
    expect(decoded["wireVersion"]).toBe(HOOK_SUBSCRIPTION_WIRE_VERSION);
    expect(decoded["token"]).toBeUndefined();
  });
});

describe("decodeSubscriptionFrame", () => {
  test("parses ready frame", () => {
    const f = rawFromString('{"frame":"ready","wireVersion":1}');
    const result = decodeSubscriptionFrame(f);
    expect(result).toEqual({ type: "Ready", wireVersion: 1 });
  });

  test("parses error frame", () => {
    const f = rawFromString('{"frame":"error","reason":"unsupported wire version"}');
    const result = decodeSubscriptionFrame(f);
    expect(result).toEqual({ type: "Error", reason: "unsupported wire version" });
  });

  test("parses Other frame for unknown discriminant", () => {
    const f = rawFromString('{"frame":"future-thing"}');
    const result = decodeSubscriptionFrame(f);
    expect(result).toEqual({ type: "Other", frame: "future-thing" });
  });

  test("returns null for invalid JSON", () => {
    const f = rawFromString("not json");
    expect(decodeSubscriptionFrame(f)).toBeNull();
  });

  test("returns null for missing frame field", () => {
    const f = rawFromString('{"wireVersion":1}');
    expect(decodeSubscriptionFrame(f)).toBeNull();
  });

  test("parses event frame and preserves correlationId", () => {
    const json = JSON.stringify({
      frame: "event",
      event: {
        sessionId: "sess-abc",
        correlationId: "corr-keep-me",
        ts: 1000,
        type: "Stop",
        payload: { turnIndex: 9 },
      },
    });
    const result = decodeSubscriptionFrame(rawFromString(json));
    expect(result?.type).toBe("Event");
    if (result?.type === "Event") {
      expect(result.event.correlationId).toBe("corr-keep-me");
      expect(result.event.sessionId).toBe("sess-abc");
    }
  });

  test("returns null for event frame with missing event field", () => {
    const f = rawFromString('{"frame":"event"}');
    expect(decodeSubscriptionFrame(f)).toBeNull();
  });
});

describe("golden fixture cross-check", () => {
  test("ready.json decodes to Ready wireVersion=1", () => {
    const result = decodeSubscriptionFrame(rawFromFile("ready.json"));
    expect(result).toEqual({ type: "Ready", wireVersion: 1 });
  });

  test("error.json decodes to Error with reason", () => {
    const result = decodeSubscriptionFrame(rawFromFile("error.json"));
    expect(result).toEqual({ type: "Error", reason: "unsupported wire version" });
  });

  test("session_start.json decodes to Event with type SessionStart", () => {
    const result = decodeSubscriptionFrame(rawFromFile("session_start.json"));
    expect(result?.type).toBe("Event");
    if (result?.type === "Event") {
      expect(result.event.type).toBe("SessionStart");
      expect(result.event.sessionId).toBe("sess-7f3a");
      expect(result.event.correlationId).toBe("corr-1");
    }
  });

  test("user_prompt_submit.json decodes to Event with type UserPromptSubmit", () => {
    const result = decodeSubscriptionFrame(rawFromFile("user_prompt_submit.json"));
    expect(result?.type).toBe("Event");
    if (result?.type === "Event") {
      expect(result.event.type).toBe("UserPromptSubmit");
    }
  });

  test("post_tool_use.json decodes to Event with type PostToolUse", () => {
    const result = decodeSubscriptionFrame(rawFromFile("post_tool_use.json"));
    expect(result?.type).toBe("Event");
    if (result?.type === "Event") {
      expect(result.event.type).toBe("PostToolUse");
    }
  });

  test("permission_request.json decodes to Event with type PermissionRequest", () => {
    const result = decodeSubscriptionFrame(rawFromFile("permission_request.json"));
    expect(result?.type).toBe("Event");
    if (result?.type === "Event") {
      expect(result.event.type).toBe("PermissionRequest");
    }
  });

  test("stop.json decodes to Event with type Stop", () => {
    const result = decodeSubscriptionFrame(rawFromFile("stop.json"));
    expect(result?.type).toBe("Event");
    if (result?.type === "Event") {
      expect(result.event.type).toBe("Stop");
    }
  });

  test("session_end.json decodes to Event with type SessionEnd", () => {
    const result = decodeSubscriptionFrame(rawFromFile("session_end.json"));
    expect(result?.type).toBe("Event");
    if (result?.type === "Event") {
      expect(result.event.type).toBe("SessionEnd");
    }
  });
});

describe("negotiateReady", () => {
  test("accepts matching wire version", () => {
    const result = negotiateReady({ type: "Ready", wireVersion: HOOK_SUBSCRIPTION_WIRE_VERSION });
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.version).toBe(HOOK_SUBSCRIPTION_WIRE_VERSION);
  });

  test("rejects mismatched wire version", () => {
    const result = negotiateReady({ type: "Ready", wireVersion: 99 });
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.kind).toBe("WireVersionMismatch");
      if (result.error.kind === "WireVersionMismatch") {
        expect(result.error.got).toBe(99);
        expect(result.error.expected).toBe(HOOK_SUBSCRIPTION_WIRE_VERSION);
      }
    }
  });

  test("returns Rejected when gate sends an error frame", () => {
    const result = negotiateReady({ type: "Error", reason: "unsupported wire version" });
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.kind).toBe("Rejected");
      if (result.error.kind === "Rejected") {
        expect(result.error.reason).toBe("unsupported wire version");
      }
    }
  });

  test("returns UnexpectedFrame for Event frame", () => {
    const result = negotiateReady({
      type: "Event",
      event: {
        sessionId: "s",
        correlationId: "c",
        ts: 0,
        type: "Stop",
        payload: {},
      },
    });
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.kind).toBe("UnexpectedFrame");
  });

  test("returns UnexpectedFrame for Other frame", () => {
    const result = negotiateReady({ type: "Other", frame: "something" });
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.kind).toBe("UnexpectedFrame");
  });
});

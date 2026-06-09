import { test, expect, describe } from "bun:test";
import {
  satisfiesMinVersion,
  assertAgentSupported,
  bootstrapAgent,
  ensureDaemon,
} from "./host-bootstrap";
import type { TauriCore, TauriChannelLike } from "./tauri";

const RANGE = ">=1.0.0";

class StubCore implements TauriCore {
  constructor(private readonly responses: Record<string, unknown | (() => unknown)>) {}
  async invoke<T = unknown>(cmd: string): Promise<T> {
    const r = this.responses[cmd];
    if (typeof r === "function") return (r as () => T)();
    if (r === undefined) throw new Error(`no stub for ${cmd}`);
    return r as T;
  }
  createChannel(): TauriChannelLike {
    return { onmessage: null };
  }
}

describe("satisfiesMinVersion", () => {
  test("accepts equal and greater, rejects lower", () => {
    expect(satisfiesMinVersion("1.0.0", RANGE)).toBe(true);
    expect(satisfiesMinVersion("1.2.3", RANGE)).toBe(true);
    expect(satisfiesMinVersion("2.0.0", RANGE)).toBe(true);
    expect(satisfiesMinVersion("0.9.9", RANGE)).toBe(false);
  });
});

describe("assertAgentSupported", () => {
  test("throws a typed VersionUnsupported below range", () => {
    expect(() => assertAgentSupported("0.9.0", RANGE)).toThrow(/does not satisfy/);
    try {
      assertAgentSupported("0.9.0", RANGE);
    } catch (e) {
      expect((e as { kind: string }).kind).toBe("VersionUnsupported");
    }
  });
  test("passes within range", () => {
    expect(() => assertAgentSupported("1.4.0", RANGE)).not.toThrow();
  });
});

describe("bootstrapAgent", () => {
  test("returns agent info when version is supported", async () => {
    const core = new StubCore({
      agent_bootstrap: {
        path: "/usr/bin/claude",
        version: "1.5.0",
        hookCommand: "/x/tillerd-notify",
      },
    });
    const info = await bootstrapAgent(core, RANGE);
    expect(info.path).toBe("/usr/bin/claude");
  });

  test("maps a resolution failure to BinaryNotFound", async () => {
    const core = new StubCore({
      agent_bootstrap: () => {
        throw new Error("claude not found on PATH");
      },
    });
    await expect(bootstrapAgent(core, RANGE)).rejects.toMatchObject({ kind: "BinaryNotFound" });
  });

  test("rejects an unsupported version before returning", async () => {
    const core = new StubCore({
      agent_bootstrap: { path: "/c", version: "0.1.0", hookCommand: null },
    });
    await expect(bootstrapAgent(core, RANGE)).rejects.toMatchObject({ kind: "VersionUnsupported" });
  });
});

describe("ensureDaemon", () => {
  test("returns ownership + socket", async () => {
    const core = new StubCore({ daemon_ensure: { ownership: "owned", socket: "/x/daemon.sock" } });
    expect(await ensureDaemon(core)).toEqual({ ownership: "owned", socket: "/x/daemon.sock" });
  });
});

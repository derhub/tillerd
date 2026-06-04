import { test, expect, describe } from "bun:test";
import { buildDesktopEngineDeps, createDesktopEngine } from "./desktop-engine";
import type { TauriCore, TauriChannelLike } from "./tauri";
import type { AgentInfo } from "./host-bootstrap";

const core: TauriCore = {
  async invoke<T = unknown>(): Promise<T> {
    return undefined as T;
  },
  createChannel(): TauriChannelLike {
    return { onmessage: null };
  },
};

const info: AgentInfo = {
  path: "/usr/bin/claude",
  version: "1.5.0",
  hookCommand: "/x/athing-notify",
  hooksSocketPath: "/x/hooks.sock",
  agentHome: "/home/u/.claude",
  homeDir: "/home/u",
};

describe("buildDesktopEngineDeps", () => {
  test("carries the resolved hooks socket and the three native ports", () => {
    const deps = buildDesktopEngineDeps(core, info);
    expect(deps.hooksSocketPath).toBe("/x/hooks.sock");
    expect(deps.agentHome).toBe("/home/u/.claude");
    expect(deps.resolvedCommand).toBe("/usr/bin/claude");
    expect(deps.transport).toBeDefined();
    expect(deps.fileSource).toBeDefined();
    expect(deps.logger).toBeDefined();
  });
});

describe("createDesktopEngine", () => {
  test("constructs an engine exposing the session API", () => {
    const engine = createDesktopEngine(core, info);
    expect(typeof engine.start).toBe("function");
    expect(typeof engine.reconnect).toBe("function");
    expect(typeof engine.listSessions).toBe("function");
  });
});

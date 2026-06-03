import { test, expect, describe } from "bun:test";
import { buildChildEnv } from "../src/pty-transport";

describe("buildChildEnv", () => {
  test("caller entries override the terminal base", () => {
    const env = buildChildEnv({ PATH: "/caller/bin", TERM: "dumb" }, "/bin/sh");
    expect(env.PATH).toBe("/caller/bin");
    expect(env.TERM).toBe("dumb");
  });

  test("terminal base is present when caller env is empty", () => {
    const env = buildChildEnv({}, "/bin/sh");
    expect(env.TERM).toBe("xterm-256color");
    expect(env.COLORTERM).toBe("truecolor");
    expect(typeof env.PATH).toBe("string");
  });

  test("caller-only variables are added", () => {
    const env = buildChildEnv({ ATHING_BRIDGE_URL: "/tmp/hooks.sock" }, "/bin/sh");
    expect(env.ATHING_BRIDGE_URL).toBe("/tmp/hooks.sock");
  });

  test("no application-specific variable is added by the base", () => {
    const env = buildChildEnv({}, "/bin/sh");
    expect(env.ATHING_BRIDGE_URL).toBeUndefined();
    expect(env.ATHING_SESSION_TOKEN).toBeUndefined();
  });
});

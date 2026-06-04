import { test, expect, describe } from "bun:test";
import { resolve } from "node:path";
import { AtError } from "@athing/sdk";
import { resolveNativeDaemonBinary, NATIVE_BUILD_OUTPUT } from "../src/resolve-daemon";

describe("native daemon resolution order", () => {
  test("explicit override wins when it exists", () => {
    const overridePath = "/opt/custom/athing-daemon";
    const resolved = resolveNativeDaemonBinary({
      env: { ATHING_DAEMON_BIN: overridePath },
      exists: (p) => p === resolve(overridePath),
    });
    expect(resolved).toBe(resolve(overridePath));
  });

  test("override is ignored when its path does not exist", () => {
    const resolved = resolveNativeDaemonBinary({
      env: { ATHING_DAEMON_BIN: "/nope/athing-daemon" },
      exists: (p) => p === NATIVE_BUILD_OUTPUT,
    });
    expect(resolved).toBe(NATIVE_BUILD_OUTPUT);
  });

  test("falls back to the native build output", () => {
    const resolved = resolveNativeDaemonBinary({
      env: {},
      exists: (p) => p === NATIVE_BUILD_OUTPUT,
    });
    expect(resolved).toBe(NATIVE_BUILD_OUTPUT);
  });

  test("falls back to an install location when no build output exists", () => {
    const resolved = resolveNativeDaemonBinary({
      env: {},
      exists: (p) => p === "/work/bin/athing-daemon",
      cwd: "/work",
    });
    expect(resolved).toBe("/work/bin/athing-daemon");
  });
});

describe("native daemon resolution failure", () => {
  test("raises a typed not-found error naming the override and the build step", () => {
    let thrown: unknown;
    try {
      resolveNativeDaemonBinary({ env: {}, exists: () => false });
    } catch (err) {
      thrown = err;
    }
    expect(thrown).toBeInstanceOf(AtError);
    expect((thrown as AtError).kind).toBe("BinaryNotFound");
    expect((thrown as AtError).message).toContain("ATHING_DAEMON_BIN");
    expect((thrown as AtError).message).toContain("cargo build");
  });
});

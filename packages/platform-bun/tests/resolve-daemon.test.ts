import { test, expect, describe } from "bun:test";
import { resolve } from "node:path";
import { resolveDaemonBinary, loginShellWhich } from "../src/supervisor";

const noWhich = () => null;

describe("reference daemon resolution order", () => {
  test("explicit override wins when it exists", () => {
    const override = "/opt/tillerd-daemon";
    const resolved = resolveDaemonBinary({
      env: { TILLERD_DAEMON_BIN: override },
      exists: (p) => p === resolve(override),
      which: noWhich,
    });
    expect(resolved).toBe(resolve(override));
  });

  test("falls back to the local bin directory", () => {
    const resolved = resolveDaemonBinary({
      env: {},
      cwd: "/work",
      exists: (p) => p === "/work/bin/tillerd-daemon",
      which: noWhich,
    });
    expect(resolved).toBe("/work/bin/tillerd-daemon");
  });

  test("falls back to the module-relative bin when local is absent", () => {
    const resolved = resolveDaemonBinary({
      env: {},
      cwd: "/nope",
      exists: (p) => p.endsWith("bin/tillerd-daemon") && !p.startsWith("/nope"),
      which: noWhich,
    });
    expect(resolved.endsWith("bin/tillerd-daemon")).toBe(true);
    expect(resolved.startsWith("/nope")).toBe(false);
  });

  test("falls back to the login-shell PATH lookup", () => {
    const resolved = resolveDaemonBinary({
      env: {},
      cwd: "/nope",
      exists: () => false,
      which: (binary) => (binary === "tillerd-daemon" ? "/usr/local/bin/tillerd-daemon" : null),
    });
    expect(resolved).toBe("/usr/local/bin/tillerd-daemon");
  });

  test("falls back to the user-local install location", () => {
    const resolved = resolveDaemonBinary({
      env: {},
      cwd: "/nope",
      home: "/home/me",
      exists: (p) => p === "/home/me/.local/bin/tillerd-daemon",
      which: noWhich,
    });
    expect(resolved).toBe("/home/me/.local/bin/tillerd-daemon");
  });
});

describe("reference daemon resolution failure", () => {
  test("throws naming the build step and the override when nothing resolves", () => {
    let thrown: unknown;
    try {
      resolveDaemonBinary({ env: {}, cwd: "/nope", exists: () => false, which: noWhich });
    } catch (err) {
      thrown = err;
    }
    expect(thrown).toBeInstanceOf(Error);
    expect((thrown as Error).message).toContain("bun run build");
    expect((thrown as Error).message).toContain("TILLERD_DAEMON_BIN");
  });
});

describe("login-shell PATH lookup", () => {
  test("resolves a binary present on PATH", () => {
    const resolved = loginShellWhich("sh");
    expect(resolved).not.toBeNull();
    expect(resolved).toContain("sh");
  });

  test("returns null for a binary absent from PATH", () => {
    expect(loginShellWhich("definitely-not-a-real-binary-xyz-123")).toBeNull();
  });
});

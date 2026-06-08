import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { resolveGateUrl, buildSetupContext } from "../src/setup";

const dirs: string[] = [];
function tempDir(): string {
  const d = fs.mkdtempSync(path.join(os.tmpdir(), "athing-gate-test-"));
  dirs.push(d);
  return d;
}

afterEach(() => {
  for (const d of dirs.splice(0)) fs.rmSync(d, { recursive: true, force: true });
});

const logger = {
  debug() {},
  info() {},
  warn() {},
  error() {},
  child() {
    return this;
  },
};

describe("resolve_gate_url_prioritizes_env_then_gate_url_file_then_undefined", () => {
  test("returns ATHING_GATE_URL env when set", () => {
    const url = resolveGateUrl({ ATHING_GATE_URL: "http://127.0.0.1:9999" });
    expect(url).toBe("http://127.0.0.1:9999");
  });

  test("reads gate.url file when env is absent", () => {
    const dir = tempDir();
    fs.writeFileSync(path.join(dir, "gate.url"), "http://127.0.0.1:8888\n", "utf8");
    const url = resolveGateUrl({ ATHING_DIR: dir });
    expect(url).toBe("http://127.0.0.1:8888");
  });

  test("returns undefined when neither env nor file is present", () => {
    const dir = tempDir();
    const url = resolveGateUrl({ ATHING_DIR: dir });
    expect(url).toBeUndefined();
  });

  test("env takes precedence over gate.url file", () => {
    const dir = tempDir();
    fs.writeFileSync(path.join(dir, "gate.url"), "http://127.0.0.1:7777", "utf8");
    const url = resolveGateUrl({ ATHING_GATE_URL: "http://127.0.0.1:9999", ATHING_DIR: dir });
    expect(url).toBe("http://127.0.0.1:9999");
  });

  test("returns undefined when gate.url file is empty", () => {
    const dir = tempDir();
    fs.writeFileSync(path.join(dir, "gate.url"), "", "utf8");
    const url = resolveGateUrl({ ATHING_DIR: dir });
    expect(url).toBeUndefined();
  });
});

describe("setup_context_carries_optional_gate_url_session_id_token", () => {
  test("buildSetupContext without gate opts has no gate fields", () => {
    const ctx = buildSetupContext("/bin/notify", logger);
    expect(ctx.gateUrl).toBeUndefined();
    expect(ctx.sessionId).toBeUndefined();
    expect(ctx.sessionToken).toBeUndefined();
  });

  test("buildSetupContext with gate opts carries all fields", () => {
    const ctx = buildSetupContext("/bin/notify", logger, {
      gateUrl: "http://127.0.0.1:9999",
      sessionId: "sid",
      sessionToken: "tok",
    });
    expect(ctx.gateUrl).toBe("http://127.0.0.1:9999");
    expect(ctx.sessionId).toBe("sid");
    expect(ctx.sessionToken).toBe("tok");
  });

  test("buildSetupContext with partial gate opts carries only what is set", () => {
    const ctx = buildSetupContext("/bin/notify", logger, { gateUrl: "http://127.0.0.1:9999" });
    expect(ctx.gateUrl).toBe("http://127.0.0.1:9999");
    expect(ctx.sessionId).toBeUndefined();
    expect(ctx.sessionToken).toBeUndefined();
  });
});

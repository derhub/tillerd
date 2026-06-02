import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { createLogger, noopLogger } from "../src/index";

function captureStdout(fn: () => void): string[] {
  const lines: string[] = [];
  const orig = process.stdout.write.bind(process.stdout);
  process.stdout.write = (chunk: string | Uint8Array) => {
    if (typeof chunk === "string") lines.push(chunk);
    return true;
  };
  fn();
  process.stdout.write = orig;
  return lines;
}

describe("createLogger", () => {
  test("emits info to stdout as JSON", () => {
    const lines = captureStdout(() => createLogger().info("hello", { x: 1 }));

    expect(lines.length).toBe(1);
    const entry = JSON.parse(lines[0]!);
    expect(entry.level).toBe("info");
    expect(entry.msg).toBe("hello");
    expect(entry.x).toBe(1);
    expect(typeof entry.ts).toBe("number");
  });

  test("emits warn to stdout as JSON with sessionId", () => {
    const lines = captureStdout(() => createLogger("sess-1").warn("bad thing"));

    const entry = JSON.parse(lines[0]!);
    expect(entry.level).toBe("warn");
    expect(entry.sessionId).toBe("sess-1");
  });

  test("suppresses debug when LOG_LEVEL=info", () => {
    const lines = captureStdout(() => createLogger().debug("should be suppressed"));
    expect(lines.length).toBe(0);
  });

  test("emits error to stdout as JSON", () => {
    const lines = captureStdout(() => createLogger().error("boom", { code: 500 }));

    const entry = JSON.parse(lines[0]!);
    expect(entry.level).toBe("error");
    expect(entry.code).toBe(500);
  });
});

describe("file logging via ATHING_DIR", () => {
  let tmpDir: string;

  afterEach(() => {
    delete process.env["ATHING_DIR"];
    if (tmpDir) fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test("writes JSON log to athing.log when ATHING_DIR is set", () => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "athing-logger-test-"));
    process.env["ATHING_DIR"] = tmpDir;

    captureStdout(() => createLogger("file-test").info("written to file", { val: 42 }));

    const date = new Date().toISOString().slice(0, 10);
    const logPath = path.join(tmpDir, "logs", `${date}.log`);
    expect(fs.existsSync(logPath)).toBe(true);
    const entry = JSON.parse(fs.readFileSync(logPath, "utf8").trim());
    expect(entry.level).toBe("info");
    expect(entry.msg).toBe("written to file");
    expect(entry.val).toBe(42);
    expect(entry.sessionId).toBe("file-test");
  });
});

describe("noopLogger", () => {
  test("all methods are no-ops", () => {
    expect(() => {
      noopLogger.debug("d");
      noopLogger.info("i");
      noopLogger.warn("w");
      noopLogger.error("e");
    }).not.toThrow();
  });
});

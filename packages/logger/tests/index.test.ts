import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { createLogger, noopLogger } from "../src/index";
import type { Resource } from "../src/index";

const RES: Resource = { "service.name": "tillerd-test", "service.version": "0.0.0" };

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
    const lines = captureStdout(() => createLogger(RES).info("hello", { x: 1 }));

    expect(lines.length).toBe(1);
    const entry = JSON.parse(lines[0]!);
    expect(entry.level).toBe("info");
    expect(entry.msg).toBe("hello");
    expect(entry.x).toBe(1);
    expect(typeof entry.ts).toBe("number");
  });

  test("stamps resource on every record", () => {
    const lines = captureStdout(() => createLogger(RES).warn("bad thing"));

    const entry = JSON.parse(lines[0]!);
    expect(entry.level).toBe("warn");
    expect(entry["service.name"]).toBe("tillerd-test");
    expect(entry["service.version"]).toBe("0.0.0");
  });

  test("suppresses debug when LOG_LEVEL=info", () => {
    const lines = captureStdout(() => createLogger(RES).debug("should be suppressed"));
    expect(lines.length).toBe(0);
  });

  test("emits error to stdout as JSON", () => {
    const lines = captureStdout(() => createLogger(RES).error("boom", { code: 500 }));

    const entry = JSON.parse(lines[0]!);
    expect(entry.level).toBe("error");
    expect(entry.code).toBe(500);
  });
});

describe("child context binding", () => {
  test("child context is inherited by every record without re-passing", () => {
    const lines = captureStdout(() => {
      const child = createLogger(RES).child({ "session.id": "s1", "pty.pid": 42 });
      child.info("spawning pty", { binary: "claude" });
    });

    const entry = JSON.parse(lines[0]!);
    expect(entry["session.id"]).toBe("s1");
    expect(entry["pty.pid"]).toBe(42);
    expect(entry.binary).toBe("claude");
    // resource still present through the child
    expect(entry["service.name"]).toBe("tillerd-test");
  });

  test("children compose", () => {
    const lines = captureStdout(() => {
      createLogger(RES).child({ component: "daemon" }).child({ "session.id": "s1" }).info("event");
    });

    const entry = JSON.parse(lines[0]!);
    expect(entry.component).toBe("daemon");
    expect(entry["session.id"]).toBe("s1");
  });

  test("inner binding wins on key collision", () => {
    const lines = captureStdout(() => {
      createLogger(RES).child({ component: "daemon" }).child({ component: "pty" }).info("event");
    });

    const entry = JSON.parse(lines[0]!);
    expect(entry.component).toBe("pty");
  });
});

describe("file logging via TILLERD_DIR", () => {
  let tmpDir: string;

  afterEach(() => {
    delete process.env["TILLERD_DIR"];
    if (tmpDir) fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test("writes one JSON record per line to the dated log file", () => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "tillerd-logger-test-"));
    process.env["TILLERD_DIR"] = tmpDir;

    captureStdout(() =>
      createLogger(RES).child({ "session.id": "file-test" }).info("written to file", { val: 42 }),
    );

    const date = new Date().toISOString().slice(0, 10);
    const logPath = path.join(tmpDir, "logs", `${date}.log`);
    expect(fs.existsSync(logPath)).toBe(true);
    const contents = fs.readFileSync(logPath, "utf8").trim();
    expect(contents.split("\n").length).toBe(1);
    const entry = JSON.parse(contents);
    expect(entry.level).toBe("info");
    expect(entry.msg).toBe("written to file");
    expect(entry.val).toBe(42);
    expect(entry["session.id"]).toBe("file-test");
    expect(entry["service.name"]).toBe("tillerd-test");
  });
});

describe("noopLogger", () => {
  test("all methods and child are no-ops", () => {
    expect(() => {
      noopLogger.debug("d");
      noopLogger.info("i");
      noopLogger.warn("w");
      noopLogger.error("e");
      noopLogger.child({ "session.id": "x" }).info("still noop");
    }).not.toThrow();
  });
});

import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { Manifest } from "../src/manifest";

function tmpDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "athing-manifest-"));
}

describe("Manifest", () => {
  const dirs: string[] = [];

  afterEach(() => {
    for (const d of dirs.splice(0)) {
      try {
        fs.rmSync(d, { recursive: true, force: true });
      } catch {}
    }
  });

  test("write then read returns same pid and version", () => {
    const dir = tmpDir();
    dirs.push(dir);
    new Manifest(dir).writeForPid(12345, "1.2.3");
    expect(Manifest.read(dir)).toEqual({ pid: 12345, version: "1.2.3" });
  });

  test("read returns null when file absent", () => {
    const dir = tmpDir();
    dirs.push(dir);
    expect(Manifest.read(dir)).toBeNull();
  });

  test("remove deletes the manifest", () => {
    const dir = tmpDir();
    dirs.push(dir);
    const m = new Manifest(dir);
    m.writeForPid(1, "0.0.1");
    m.remove();
    expect(Manifest.read(dir)).toBeNull();
  });

  test("remove is idempotent when file absent", () => {
    const dir = tmpDir();
    dirs.push(dir);
    const m = new Manifest(dir);
    expect(() => m.remove()).not.toThrow();
    expect(() => m.remove()).not.toThrow();
  });

  test("write is atomic — tmp file gone after write", () => {
    const dir = tmpDir();
    dirs.push(dir);
    const m = new Manifest(dir);
    m.writeForPid(99, "0.0.1");
    expect(fs.existsSync(path.join(dir, "daemon.json"))).toBe(true);
    expect(fs.existsSync(path.join(dir, "daemon.json.tmp"))).toBe(false);
  });

  test("write creates directory if missing", () => {
    const dir = tmpDir();
    dirs.push(dir);
    const nested = path.join(dir, "deep", "nested");
    new Manifest(nested).writeForPid(1, "0.0.1");
    expect(Manifest.read(nested)).toEqual({ pid: 1, version: "0.0.1" });
  });
});

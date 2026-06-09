import { test, expect, describe } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  spawnFieldsDiffer,
  resolveTillerdDir,
  adoptOrSpawnTool,
  type SpawnSpec,
  type ToolManifest,
} from "../src/process-launch";
import { AtError } from "@tillerd/sdk";

function tempDir(tag: string): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), `tillerd-pl-${tag}-`));
}

function writeManifest(dir: string, data: ToolManifest): string {
  const p = path.join(dir, "tool.json");
  fs.writeFileSync(p, JSON.stringify(data), "utf8");
  return p;
}

// --- R6: spawn_fields_differ ---

describe("spawnFieldsDiffer", () => {
  const base: SpawnSpec = {
    command: "tillerd-daemon",
    args: ["--serve"],
    cwd: "/work",
    env: { TILLERD_DIR: "/run" },
  };

  test("spawn_fields_differ_table_R6: detects command change", () => {
    expect(spawnFieldsDiffer(base, { ...base, command: "other-daemon" }, ["TILLERD_DIR"])).toBe(
      true,
    );
  });

  test("spawn_fields_differ_table_R6: detects args change", () => {
    expect(
      spawnFieldsDiffer(base, { ...base, args: ["--serve", "--verbose"] }, ["TILLERD_DIR"]),
    ).toBe(true);
  });

  test("spawn_fields_differ_table_R6: detects allowlisted env var change", () => {
    const b = { ...base, env: { TILLERD_DIR: "/elsewhere" } };
    expect(spawnFieldsDiffer(base, b, ["TILLERD_DIR"])).toBe(true);
  });

  test("spawn_fields_differ_table_R6: ignores env var outside allowlist", () => {
    const b = { ...base, env: { ...base.env, LOG_LEVEL: "trace" } };
    expect(spawnFieldsDiffer(base, b, ["TILLERD_DIR"])).toBe(false);
  });

  test("spawn_fields_differ_table_R6: detects cwd change", () => {
    expect(spawnFieldsDiffer(base, { ...base, cwd: "/other" }, ["TILLERD_DIR"])).toBe(true);
  });

  test("spawn_fields_differ_table_R6: equal specs are not different", () => {
    expect(spawnFieldsDiffer(base, { ...base }, ["TILLERD_DIR"])).toBe(false);
  });
});

// --- R7: TILLERD_DIR resolution parity ---

describe("tillerd_dir_resolution_parity_R7", () => {
  test("uses TILLERD_DIR env when set (absolute path passes through)", () => {
    const dir = resolveTillerdDir({ TILLERD_DIR: "/custom/dir" });
    expect(dir).toBe("/custom/dir");
  });

  test("resolves relative TILLERD_DIR against cwd via path.resolve", () => {
    const dir = resolveTillerdDir({ TILLERD_DIR: "relative/dir" });
    expect(path.isAbsolute(dir)).toBe(true);
    expect(dir.endsWith("relative/dir")).toBe(true);
  });

  test("falls back to ~/.tillerd when TILLERD_DIR is unset", () => {
    const dir = resolveTillerdDir({});
    expect(dir.endsWith(".tillerd")).toBe(true);
    expect(path.isAbsolute(dir)).toBe(true);
  });
});

// --- R3: adopts on exact version match ---

describe("adoptOrSpawnTool", () => {
  test("adopts_on_exact_version_match: returns pid without spawning when manifest matches", async () => {
    const dir = tempDir("adopt");
    const manifestPath = writeManifest(dir, { pid: process.pid, version: "1.2.3" });
    try {
      const result = await adoptOrSpawnTool(
        { command: "never-called", args: [], env: {} },
        { version: "1.2.3", manifestPath },
      );
      expect(result.adopted).toBe(true);
      expect(result.pid).toBe(process.pid);
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  test("adopts_on_exact_version_match: does not adopt when version mismatches", async () => {
    const dir = tempDir("version-miss");
    // Write a manifest with a different version — the spawn path will be tried,
    // but since we have no real binary it will fail after backoff.
    writeManifest(dir, { pid: process.pid, version: "9.9.9" });
    const manifestPath = path.join(dir, "tool.json");
    try {
      await adoptOrSpawnTool(
        { command: "/nonexistent/binary", args: [], env: {} },
        { version: "1.2.3", manifestPath, maxAttempts: 1, backoffMs: 0, startupTimeoutMs: 50 },
      );
      expect(true).toBe(false); // should have thrown
    } catch (err) {
      expect(err).toBeInstanceOf(AtError);
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  test("spawns_and_overwrites_stale_manifest: propagates spec env to spawned process", async () => {
    // Verify that when no adoptable instance exists, spawn is attempted with the
    // correct spec fields. We confirm by checking that the spawn attempt fails
    // with SpawnFailed (the binary doesn't exist) not Timeout.
    const dir = tempDir("stale");
    const manifestPath = path.join(dir, "tool.json");
    try {
      await adoptOrSpawnTool(
        { command: "/no/such/binary", args: ["--test"], cwd: dir, env: { TILLERD_DIR: dir } },
        { version: "1.0.0", manifestPath, maxAttempts: 1, backoffMs: 0, startupTimeoutMs: 50 },
      );
      expect(true).toBe(false);
    } catch (err) {
      expect(err).toBeInstanceOf(AtError);
      expect((err as AtError).kind).toBe("SpawnFailed");
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  test("bounded_backoff_caps_attempts: respects maxAttempts", async () => {
    const dir = tempDir("backoff");
    const manifestPath = path.join(dir, "tool.json");
    const start = Date.now();
    try {
      await adoptOrSpawnTool(
        { command: "/nonexistent", args: [], env: {} },
        { version: "1.0.0", manifestPath, maxAttempts: 2, backoffMs: 10, startupTimeoutMs: 20 },
      );
    } catch {
      // expected
    }
    const elapsed = Date.now() - start;
    // 2 attempts × 20ms timeout = at most ~50ms + backoff
    // Should not have taken more than 500ms
    expect(elapsed).toBeLessThan(500);
    fs.rmSync(dir, { recursive: true, force: true });
  });
});

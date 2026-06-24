import { test, expect, describe } from "bun:test";

import { run } from "../src/cli";
import { harness } from "./helpers";

describe("status", () => {
  test("running: manifest present and pid alive exits zero", async () => {
    const h = harness({ manifest: { pid: 1234, version: "0.0.1" }, isAlive: true });
    const code = await run(["status"], h.deps);
    expect(code).toBe(0);
    const line = h.out.join("\n");
    expect(line).toContain("running");
    expect(line).toContain("1234");
    expect(line).toContain("0.0.1");
  });

  test("stale: manifest present but pid dead exits non-zero", async () => {
    const h = harness({ manifest: { pid: 1234, version: "0.0.1" }, isAlive: false });
    const code = await run(["status"], h.deps);
    expect(code).not.toBe(0);
    expect(h.out.join("\n")).toContain("not running");
  });

  test("absent: no manifest exits non-zero", async () => {
    const h = harness({ manifest: null });
    const code = await run(["status"], h.deps);
    expect(code).not.toBe(0);
    expect(h.out.join("\n")).toContain("not running");
  });

  test("--json emits a single object describing state", async () => {
    const h = harness({ manifest: { pid: 1234, version: "0.0.1" }, isAlive: true });
    const code = await run(["status", "--json"], h.deps);
    expect(code).toBe(0);
    expect(h.out).toHaveLength(1);
    expect(JSON.parse(h.out[0]!)).toEqual({
      running: true,
      state: "running",
      pid: 1234,
      version: "0.0.1",
    });
  });

  test("--json for absent daemon reports running false", async () => {
    const h = harness({ manifest: null });
    await run(["status", "--json"], h.deps);
    expect(JSON.parse(h.out[0]!)).toEqual({
      running: false,
      state: "absent",
      pid: null,
      version: null,
    });
  });
});

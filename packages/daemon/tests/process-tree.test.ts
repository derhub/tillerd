import { test, expect, describe } from "bun:test";
import { parsePsOutput, collectDescendantPids } from "../src/process-tree";

describe("parsePsOutput", () => {
  test("parses pid/ppid pairs with ps leading-space padding", () => {
    const output = "  101   1\n  202 101\n 3030 202\n";
    expect(parsePsOutput(output)).toEqual([
      { pid: 101, ppid: 1 },
      { pid: 202, ppid: 101 },
      { pid: 3030, ppid: 202 },
    ]);
  });

  test("ignores blank and malformed lines", () => {
    const output = "\n  101   1\ngarbage\n  202\n  303 abc\n";
    expect(parsePsOutput(output)).toEqual([{ pid: 101, ppid: 1 }]);
  });

  test("returns empty array for empty input", () => {
    expect(parsePsOutput("")).toEqual([]);
  });
});

describe("collectDescendantPids", () => {
  test("collects a deep chain, excluding the root", () => {
    const procs = [
      { pid: 100, ppid: 1 },
      { pid: 200, ppid: 100 },
      { pid: 300, ppid: 200 },
    ];
    expect(collectDescendantPids(100, procs).sort()).toEqual([200, 300]);
  });

  test("collects multiple branches", () => {
    const procs = [
      { pid: 100, ppid: 1 },
      { pid: 200, ppid: 100 },
      { pid: 201, ppid: 100 },
      { pid: 300, ppid: 200 },
    ];
    expect(collectDescendantPids(100, procs).sort()).toEqual([200, 201, 300]);
  });

  test("catches a setsid child whose group detached but parent persists", () => {
    // 400 changed its process group (would escape kill(-100)) but ppid stays 100.
    const procs = [
      { pid: 100, ppid: 1 },
      { pid: 400, ppid: 100 },
    ];
    expect(collectDescendantPids(100, procs)).toContain(400);
  });

  test("ignores unrelated processes", () => {
    const procs = [
      { pid: 100, ppid: 1 },
      { pid: 200, ppid: 100 },
      { pid: 999, ppid: 5 },
    ];
    expect(collectDescendantPids(100, procs)).toEqual([200]);
  });

  test("terminates on a pid-reuse cycle", () => {
    const procs = [
      { pid: 100, ppid: 200 },
      { pid: 200, ppid: 100 },
    ];
    expect(collectDescendantPids(100, procs)).toEqual([200]);
  });

  test("returns empty when root has no children", () => {
    expect(collectDescendantPids(100, [{ pid: 100, ppid: 1 }])).toEqual([]);
  });
});

import { test, expect, describe } from "bun:test";
import { run, USAGE } from "../src/cli";
import { harness } from "./helpers";

describe("command routing", () => {
  test("no subcommand prints usage to stderr and exits non-zero", async () => {
    const h = harness();
    const code = await run([], h.deps);
    expect(code).not.toBe(0);
    expect(h.err.join("\n")).toContain(USAGE);
  });

  test("unknown subcommand is rejected with its name and usage", async () => {
    const h = harness();
    const code = await run(["bogus"], h.deps);
    expect(code).not.toBe(0);
    expect(h.err.join("\n")).toContain("bogus");
    expect(h.err.join("\n")).toContain(USAGE);
  });

  test("--help prints usage to stdout and exits zero", async () => {
    const h = harness();
    const code = await run(["--help"], h.deps);
    expect(code).toBe(0);
    expect(h.out.join("\n")).toContain(USAGE);
  });

  test("known subcommand routes to its handler", async () => {
    const h = harness({ manifest: null });
    const code = await run(["status"], h.deps);
    expect(code).not.toBe(0); // daemon absent -> non-zero
  });
});

describe("argument validation", () => {
  test("unknown flag is rejected", async () => {
    const h = harness({ manifest: null });
    const code = await run(["status", "--bogus"], h.deps);
    expect(code).not.toBe(0);
    expect(h.err.join("\n")).toContain("invalid arguments");
  });

  test("extra positional is rejected", async () => {
    const h = harness({ manifest: null });
    const code = await run(["status", "wat"], h.deps);
    expect(code).not.toBe(0);
    expect(h.err.join("\n")).toContain("unexpected argument: wat");
  });

  test("leading flag with no subcommand prints usage non-zero", async () => {
    const h = harness();
    const code = await run(["--frobnicate"], h.deps);
    expect(code).not.toBe(0);
  });
});

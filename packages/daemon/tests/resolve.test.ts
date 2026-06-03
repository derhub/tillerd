import { test, expect, describe } from "bun:test";
import { resolveCommand } from "../src/resolve";

describe("resolveCommand", () => {
  test("absolute path is returned as-is", () => {
    expect(resolveCommand("/usr/bin/env")).toBe("/usr/bin/env");
  });

  test("no command defaults to the login shell", () => {
    const prev = process.env["SHELL"];
    process.env["SHELL"] = "/bin/zsh";
    try {
      expect(resolveCommand()).toBe("/bin/zsh");
    } finally {
      if (prev === undefined) delete process.env["SHELL"];
      else process.env["SHELL"] = prev;
    }
  });
});

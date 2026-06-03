import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { checkCliVersion, resolveBinary } from "../src/resolve";
import { prepareNotifyScript } from "../src/ingress";

// The host bootstrap must surface typed errors before the engine accepts sessions.

describe("startup bootstrap — typed errors", () => {
  const savedAthingDir = process.env["ATHING_DIR"];
  const savedExec = process.env["CLAUDE_CODE_EXECUTABLE"];

  afterEach(() => {
    if (savedAthingDir === undefined) delete process.env["ATHING_DIR"];
    else process.env["ATHING_DIR"] = savedAthingDir;
    if (savedExec === undefined) delete process.env["CLAUDE_CODE_EXECUTABLE"];
    else process.env["CLAUDE_CODE_EXECUTABLE"] = savedExec;
  });

  test("checkCliVersion throws VersionUnsupported when installed version is below range", () => {
    // `bun --version` is present and parseable; an impossible floor forces the failure path.
    expect(() => checkCliVersion("bun", ">=999.0.0")).toThrow(/VersionUnsupported|does not satisfy/);
  });

  test("checkCliVersion passes for an open range", () => {
    expect(() => checkCliVersion("bun", "*")).not.toThrow();
  });

  test("prepareNotifyScript throws HookInstallFailed when the notify client is absent", () => {
    const missing = path.join(fs.mkdtempSync(path.join(os.tmpdir(), "athing-boot-")), "athing-notify");
    expect(() => prepareNotifyScript(missing)).toThrow(/not found|HookInstallFailed/);
  });

  test("resolveBinary throws BinaryNotFound for an unresolvable command", () => {
    delete process.env["CLAUDE_CODE_EXECUTABLE"];
    expect(() => resolveBinary("definitely-not-a-real-binary-xyz123")).toThrow(
      /Cannot resolve|BinaryNotFound/,
    );
  });
});

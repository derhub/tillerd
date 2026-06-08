import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { checkCliVersion, resolveAgentCommand } from "../src/resolve";
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
    expect(() => checkCliVersion("bun", ">=999.0.0")).toThrow(
      /VersionUnsupported|does not satisfy/,
    );
  });

  test("checkCliVersion passes for an open range", () => {
    expect(() => checkCliVersion("bun", "*")).not.toThrow();
  });

  test("prepareNotifyScript throws HookInstallFailed when the notify client is absent", () => {
    const missing = path.join(
      fs.mkdtempSync(path.join(os.tmpdir(), "athing-boot-")),
      "athing-notify",
    );
    expect(() => prepareNotifyScript(missing)).toThrow(/not found|HookInstallFailed/);
  });

  test("resolveAgentCommand throws BinaryNotFound for an unresolvable command", () => {
    expect(() =>
      resolveAgentCommand({
        overrideEnvVar: "NOT_A_REAL_OVERRIDE",
        binaryName: "definitely-not-a-real-binary-xyz123",
        commonLocations: [],
      }),
    ).toThrow(/Cannot resolve|BinaryNotFound/);
  });

  test("resolveAgentCommand honors the override env var", () => {
    process.env["MY_AGENT_OVERRIDE"] = "/opt/custom/agent";
    expect(
      resolveAgentCommand({
        overrideEnvVar: "MY_AGENT_OVERRIDE",
        binaryName: "agent",
        commonLocations: [],
      }),
    ).toBe("/opt/custom/agent");
    delete process.env["MY_AGENT_OVERRIDE"];
  });
});

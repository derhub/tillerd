import { test, expect, describe, afterEach } from "bun:test";
import * as os from "node:os";
import * as path from "node:path";
import { notifyCommand, notifyScriptPath, prepareNotifyScript } from "../src/ingress";

const NOTIFY_BIN = path.join(import.meta.dir, "../../../bin/tillerd-notify");

describe("notify client resolution", () => {
  const prev = process.env["TILLERD_NOTIFY_BIN"];
  afterEach(() => {
    if (prev === undefined) delete process.env["TILLERD_NOTIFY_BIN"];
    else process.env["TILLERD_NOTIFY_BIN"] = prev;
  });

  test("notifyCommand resolves the committed bin path", () => {
    process.env["TILLERD_NOTIFY_BIN"] = NOTIFY_BIN;
    expect(notifyCommand()).toBe(path.resolve(NOTIFY_BIN));
    expect(notifyScriptPath()).toBe(path.resolve(NOTIFY_BIN));
  });

  test("prepareNotifyScript returns the executable command", () => {
    process.env["TILLERD_NOTIFY_BIN"] = NOTIFY_BIN;
    expect(prepareNotifyScript()).toEqual({ command: path.resolve(NOTIFY_BIN), updated: false });
  });

  test("prepareNotifyScript throws HookInstallFailed when the client is absent", () => {
    const missing = path.join(os.tmpdir(), "tillerd-nonexistent", "tillerd-notify");
    expect(() => prepareNotifyScript(missing)).toThrow(/not found/);
  });
});

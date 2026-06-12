#!/usr/bin/env bun
import { confirm, isCancel } from "@clack/prompts";
import { defineSetup } from "@tillerd/sdk";
import { run, type CliDeps } from "./cli";

const deps: CliDeps = {
  setup: defineSetup({ async install() {}, async uninstall() {} }),
  buildContext: (notifyCommand, logger) => ({
    notifyCommand,
    agentHome: process.env["HOME"] ? `${process.env["HOME"]}/.claude` : "",
    logger,
    fs: {
      async readText() {
        return null;
      },
      async writeAtomic() {},
      async backup() {},
      async exists() {
        return false;
      },
    },
  }),
  resolveNotify: () => {
    throw new Error("gate-notify binary resolution not available in this build");
  },
  readManifest: () => null,
  isAlive: () => false,
  isTTY: Boolean(process.stdin.isTTY),
  async confirm(message) {
    const res = await confirm({ message });
    return !isCancel(res) && res === true;
  },
  out: (line) => console.log(line),
  err: (line) => console.error(line),
};

run(process.argv.slice(2), deps).then((code) => process.exit(code));

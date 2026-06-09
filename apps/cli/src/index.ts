#!/usr/bin/env bun
import { confirm, isCancel } from "@clack/prompts";
import { setup } from "@athing/adapter-claude-code";
import {
  buildSetupContext,
  isAlive,
  prepareNotifyScript,
  readManifest,
} from "@athing/platform-bun";
import { run, type CliDeps } from "./cli";

const deps: CliDeps = {
  setup,
  buildContext: (notifyCommand, logger) => buildSetupContext(notifyCommand, logger),
  resolveNotify: () => prepareNotifyScript().command,
  readManifest: () => readManifest(),
  isAlive,
  isTTY: Boolean(process.stdin.isTTY),
  async confirm(message) {
    const res = await confirm({ message });
    return !isCancel(res) && res === true;
  },
  out: (line) => console.log(line),
  err: (line) => console.error(line),
};

run(process.argv.slice(2), deps).then((code) => process.exit(code));

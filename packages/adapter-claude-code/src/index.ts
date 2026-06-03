import type { AgentDefinition } from "@athing/sdk";
import { installHooks, uninstallHooks } from "./hook-installer";
import { parseHook } from "./parse-hook";
import { transcriptPath } from "./transcript-path";
import { parseTranscriptEntry } from "./parse-entry";
import { resolveAgentBinary } from "./resolve";

export const SUPPORTED_CLI_VERSION_RANGE = ">=1.0.0";

const INTERRUPT_SEQUENCE = "\x1b";

export const claudeCode: AgentDefinition = {
  name: "claude-code",
  launch: {
    command: "claude",
    args: ["--session-id", "{id}"],
    flags: ["--dangerously-skip-permissions"],
  },
  cliVersionRange: SUPPORTED_CLI_VERSION_RANGE,
  interruptSequence: INTERRUPT_SEQUENCE,
  resolveCommand: () => resolveAgentBinary("claude"),
  installHooks,
  uninstallHooks,
  parseHook,
  transcriptPath,
  parseTranscriptEntry: (line) => parseTranscriptEntry(line),
};

export { parseHook, transcriptPath, parseTranscriptEntry, resolveAgentBinary };

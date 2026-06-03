import type { AgentDefinition } from "@athing/sdk";
import { BINARY_RESOLUTION } from "./binary-resolution";
import { parseHook } from "./parse-hook";
import { transcriptPath } from "./transcript-path";
import { parseTranscriptEntry } from "./parse-entry";

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
  binaryResolution: BINARY_RESOLUTION,
  parseHook,
  transcriptPath,
  parseTranscriptEntry: (line) => parseTranscriptEntry(line),
};

export { setup } from "./setup";
export { parseHook, transcriptPath, parseTranscriptEntry, BINARY_RESOLUTION };

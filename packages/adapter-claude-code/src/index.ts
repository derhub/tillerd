import * as os from "node:os";
import * as path from "node:path";
import type { AgentDefinition } from "@athing/sdk";
import { parseHook } from "./parse-hook";
import { transcriptPath } from "./transcript-path";
import { parseTranscriptEntry } from "./parse-entry";

export const SUPPORTED_CLI_VERSION_RANGE = ">=1.0.0";

export const claudeCode: AgentDefinition = {
  name: "claude-code",
  launch: {
    command: "claude",
    args: ["--session-id", "{id}"],
    flags: ["--dangerously-skip-permissions"],
  },
  hookInstall: {
    settingsPath: "~/.claude/settings.json",
    notifyScriptPath: path.join(os.homedir(), ".athing", "notify.mjs"),
    events: [
      "SessionStart",
      "UserPromptSubmit",
      "PostToolUse",
      "PermissionRequest",
      "Stop",
      "SessionEnd",
    ],
  },
  cliVersionRange: SUPPORTED_CLI_VERSION_RANGE,
  parseHook,
  transcriptPath,
  parseTranscriptEntry: (line) => parseTranscriptEntry(line),
};

export { parseHook, transcriptPath, parseTranscriptEntry };

import type { AgentDefinition } from "@athing/sdk";

export const bashAdapter: AgentDefinition = {
  name: "bash",
  launch: { command: "bash", args: [], flags: [] },
  cliVersionRange: "*",
  installHooks: () => {},
  uninstallHooks: () => {},
  parseHook: () => {
    throw new Error("bash adapter does not emit hooks");
  },
  transcriptPath: (sessionId) => `/tmp/athing-bash-${sessionId}.jsonl`,
  parseTranscriptEntry: () => null,
};

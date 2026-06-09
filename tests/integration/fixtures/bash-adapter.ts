import type { AgentDefinition } from "@tillerd/sdk";

export const bashAdapter: AgentDefinition = {
  name: "bash",
  launch: { command: "bash", args: [], flags: [] },
  interruptSequence: "\x1b",
  cliVersionRange: "*",
  binaryResolution: { overrideEnvVar: "BASH_BIN", binaryName: "bash", commonLocations: [] },
};

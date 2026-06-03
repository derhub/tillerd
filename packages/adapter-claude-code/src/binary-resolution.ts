import type { BinaryResolutionSpec } from "@athing/sdk";

export const BINARY_RESOLUTION: BinaryResolutionSpec = {
  overrideEnvVar: "CLAUDE_CODE_EXECUTABLE",
  binaryName: "claude",
  commonLocations: [
    "/usr/local/bin/claude",
    "/usr/bin/claude",
    "~/.local/bin/claude",
    "~/.npm-global/bin/claude",
  ],
};

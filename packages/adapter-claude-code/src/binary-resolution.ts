import type { BinaryResolutionSpec } from "@tillerd/sdk";

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

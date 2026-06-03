import { AtError } from "@athing/sdk";
import { spawnSync } from "node:child_process";
import { statSync } from "node:fs";

const COMMON_LOCATIONS = [
  "/usr/local/bin/claude",
  "/usr/bin/claude",
  `${process.env["HOME"] ?? ""}/.local/bin/claude`,
  `${process.env["HOME"] ?? ""}/.npm-global/bin/claude`,
];

function loginShellWhich(binary: string): string | null {
  const shell = process.env["SHELL"] ?? "/bin/sh";
  const result = spawnSync(shell, ["-lc", `which ${binary}`], {
    encoding: "utf8",
    timeout: 5000,
  });
  if (result.status === 0) {
    const resolved = result.stdout.trim();
    if (resolved) return resolved;
  }
  return null;
}

/**
 * Resolve the agent binary to a launchable command. Agent-specific resolution
 * (override env var, then login-shell PATH, then common install locations) lives
 * with the adapter so the daemon stays a generic terminal backend.
 */
export function resolveAgentBinary(command: string): string {
  const envOverride = process.env["CLAUDE_CODE_EXECUTABLE"];
  if (envOverride) return envOverride;

  const fromShell = loginShellWhich(command);
  if (fromShell) return fromShell;

  for (const loc of COMMON_LOCATIONS) {
    try {
      if (statSync(loc).isFile()) return loc;
    } catch {
      // not found
    }
  }

  throw new AtError(
    "BinaryNotFound",
    `Cannot resolve '${command}'. Set CLAUDE_CODE_EXECUTABLE or ensure it is on PATH.`,
  );
}

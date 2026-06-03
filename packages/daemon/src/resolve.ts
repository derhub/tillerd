import { AtError } from "@athing/sdk";
import { spawnSync } from "node:child_process";

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
 * Generic command resolution for the terminal backend. An absolute path is used
 * as given; a bare name is resolved via the login-shell PATH; when no command is
 * supplied the user's login shell is used. The daemon carries no application
 * default command, install location, or version gate.
 */
export function resolveCommand(command?: string): string {
  if (!command) return process.env["SHELL"] ?? "/bin/sh";
  if (command.startsWith("/")) return command;

  const fromShell = loginShellWhich(command);
  if (fromShell) return fromShell;

  throw new AtError(
    "BinaryNotFound",
    `Cannot resolve '${command}'. Provide an absolute path or ensure it is on PATH.`,
  );
}

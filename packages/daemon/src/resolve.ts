import * as fs from "node:fs";
import { AtError } from "@athing/sdk";
import { spawnSync } from "node:child_process";

export function checkCliVersion(command: string, versionRange: string): void {
  if (!versionRange || versionRange === "*") return;
  const shell = process.env["SHELL"] ?? "/bin/sh";
  const result = spawnSync(shell, ["-lc", `${command} --version`], {
    encoding: "utf8",
    timeout: 5000,
  });
  if (result.status !== 0 || !result.stdout) return;
  const match = result.stdout.match(/(\d+)\.(\d+)\.(\d+)/);
  if (!match) return;
  const installed = `${match[1]!}.${match[2]!}.${match[3]!}`;
  if (!satisfiesRange(installed, versionRange)) {
    throw new AtError(
      "VersionUnsupported",
      `Installed ${command} ${installed} does not satisfy ${versionRange}`,
    );
  }
}

export function satisfiesRange(version: string, range: string): boolean {
  if (range === "*") return true;
  const m = range.match(/^(>=|<=|>|<|\^|~)?(\d+)\.(\d+)\.(\d+)/);
  if (!m) return true;
  const op = m[1] ?? ">=";
  const [rmaj, rmin, rpat] = [Number(m[2]), Number(m[3]), Number(m[4])];
  const vm = version.match(/(\d+)\.(\d+)\.(\d+)/);
  if (!vm) return false;
  const [vmaj, vmin, vpat] = [Number(vm[1]), Number(vm[2]), Number(vm[3])];
  const diff = vmaj !== rmaj ? vmaj - rmaj : vmin !== rmin ? vmin - rmin : vpat - rpat;
  switch (op) {
    case ">=":
      return diff >= 0;
    case ">":
      return diff > 0;
    case "<=":
      return diff <= 0;
    case "<":
      return diff < 0;
    case "^":
      return vmaj === rmaj && diff >= 0;
    case "~":
      return vmaj === rmaj && vmin === rmin && diff >= 0;
    default:
      return diff >= 0;
  }
}

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

export function resolveBinary(command: string): string {
  const envOverride = process.env["CLAUDE_CODE_EXECUTABLE"];
  if (envOverride) return envOverride;

  const fromShell = loginShellWhich(command);
  if (fromShell) return fromShell;

  if (command === "claude") {
    for (const loc of COMMON_LOCATIONS) {
      try {
        const stat = fs.statSync(loc);
        if (stat.isFile()) return loc;
      } catch {
        // not found
      }
    }
  }

  throw new AtError(
    "BinaryNotFound",
    `Cannot resolve '${command}'. Set CLAUDE_CODE_EXECUTABLE or ensure it is on PATH.`,
  );
}

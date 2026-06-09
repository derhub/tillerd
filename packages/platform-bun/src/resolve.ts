import { AtError } from "@tillerd/sdk";
import type { BinaryResolutionSpec } from "@tillerd/sdk";
import { spawnSync } from "node:child_process";
import { statSync } from "node:fs";
import { homedir } from "node:os";

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

function satisfiesRange(version: string, range: string): boolean {
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

function expandHome(location: string): string {
  return location.startsWith("~/") ? `${homedir()}/${location.slice(2)}` : location;
}

/**
 * Resolve the agent binary to a launchable command using the adapter's declarative
 * policy: an explicit env override, then the login-shell PATH, then the common
 * install locations. The resolution I/O lives in the host; the adapter owns only
 * the policy data.
 */
export function resolveAgentCommand(spec: BinaryResolutionSpec): string {
  const override = process.env[spec.overrideEnvVar];
  if (override) return override;

  const fromShell = loginShellWhich(spec.binaryName);
  if (fromShell) return fromShell;

  for (const loc of spec.commonLocations) {
    const abs = expandHome(loc);
    try {
      if (statSync(abs).isFile()) return abs;
    } catch {
      // not found
    }
  }

  throw new AtError(
    "BinaryNotFound",
    `Cannot resolve '${spec.binaryName}'. Set ${spec.overrideEnvVar} or ensure it is on PATH.`,
  );
}

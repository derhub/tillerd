import { execFileSync } from "node:child_process";

const PROBE_TIMEOUT_MS = 5_000;

const ENV_NAMES = [
  "PATH",
  "SSH_AUTH_SOCK",
  "HOMEBREW_PREFIX",
  "HOMEBREW_CELLAR",
  "HOMEBREW_REPOSITORY",
  "XDG_CONFIG_HOME",
  "XDG_DATA_HOME",
] as const;

type EnvName = (typeof ENV_NAMES)[number];

function startMarker(name: string): string {
  return `__ATHING_ENV_${name}_START__`;
}

function endMarker(name: string): string {
  return `__ATHING_ENV_${name}_END__`;
}

function buildCaptureCommand(names: readonly string[]): string {
  return names
    .map((name) =>
      [
        `printf '%s\\n' '${startMarker(name)}'`,
        `printenv ${name} || true`,
        `printf '%s\\n' '${endMarker(name)}'`,
      ].join("; "),
    )
    .join("; ");
}

function extractValue(output: string, name: string): string | undefined {
  const start = output.indexOf(startMarker(name));
  if (start === -1) return undefined;
  const valueStart = start + startMarker(name).length;
  const end = output.indexOf(endMarker(name), valueStart);
  if (end === -1) return undefined;
  const value = output.slice(valueStart, end).replace(/^\r?\n/, "").replace(/\r?\n$/, "");
  return value.length > 0 ? value : undefined;
}

function shellCandidates(): string[] {
  const candidates: string[] = [];
  if (process.env["SHELL"]) candidates.push(process.env["SHELL"]);
  if (process.platform === "darwin") {
    candidates.push("/bin/zsh", "/bin/bash");
  } else {
    candidates.push("/bin/bash", "/bin/sh");
  }
  return [...new Set(candidates)];
}

function probeShell(shell: string): Partial<Record<EnvName, string>> {
  const output = execFileSync(shell, ["-ilc", buildCaptureCommand(ENV_NAMES)], {
    encoding: "utf8",
    timeout: PROBE_TIMEOUT_MS,
  });
  const result: Partial<Record<EnvName, string>> = {};
  for (const name of ENV_NAMES) {
    const value = extractValue(output, name);
    if (value !== undefined) result[name] = value;
  }
  return result;
}

export function installLoginShellEnv(): void {
  for (const shell of shellCandidates()) {
    try {
      const env = probeShell(shell);
      if (!env.PATH) continue;

      // PATH: overwrite with login shell value (superset of daemon's minimal PATH)
      process.env["PATH"] = env.PATH;

      // Other vars: only set if not already present
      for (const name of ENV_NAMES) {
        if (name === "PATH") continue;
        const value = env[name];
        if (value && !process.env[name]) {
          process.env[name] = value;
        }
      }
      return;
    } catch {
      continue;
    }
  }
}

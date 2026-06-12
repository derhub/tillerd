import { parseArgs, type ParseArgsConfig } from "util";

export interface ManifestData {
  pid: number;
  version: string;
}

/**
 * Everything the CLI touches, injected so the core is exercised against fixtures
 * and fakes — never the operator's real manifest file.
 */
export interface CliDeps {
  readManifest(): ManifestData | null;
  isAlive(pid: number): boolean;
  out(line: string): void;
  err(line: string): void;
}

export const USAGE = `tillerd — daemon controller

Usage:
  tillerd status [--json]    Report whether the daemon is running

Flags:
  --json        Machine-readable output
  -h, --help    Show this help`;

type OptionSpec = ParseArgsConfig["options"];

const COMMAND_OPTIONS: Record<string, OptionSpec> = {
  status: { json: { type: "boolean", default: false } },
};

export async function run(argv: string[], deps: CliDeps): Promise<number> {
  if (argv.includes("-h") || argv.includes("--help")) {
    deps.out(USAGE);
    return 0;
  }

  const sub = argv[0];
  if (!sub || sub.startsWith("-")) {
    deps.err(USAGE);
    return 1;
  }

  const options = COMMAND_OPTIONS[sub];
  if (!options) {
    deps.err(`unknown subcommand: ${sub}`);
    deps.err(USAGE);
    return 1;
  }

  let values: Record<string, unknown>;
  let positionals: string[];
  try {
    ({ values, positionals } = parseArgs({
      args: argv.slice(1),
      options,
      allowPositionals: true,
      strict: true,
    }));
  } catch (e) {
    deps.err(`invalid arguments: ${(e as Error).message}`);
    deps.err(USAGE);
    return 1;
  }

  if (positionals.length > 0) {
    deps.err(`unexpected argument: ${positionals[0]}`);
    deps.err(USAGE);
    return 1;
  }

  return status(deps, Boolean(values["json"]));
}

function status(deps: CliDeps, json: boolean): number {
  const manifest = deps.readManifest();
  const running = manifest !== null && deps.isAlive(manifest.pid);
  const state = manifest === null ? "absent" : running ? "running" : "stale";

  if (json) {
    deps.out(
      JSON.stringify({
        running,
        state,
        pid: manifest?.pid ?? null,
        version: manifest?.version ?? null,
      }),
    );
  } else if (state === "running") {
    deps.out(`daemon running (pid ${manifest!.pid}, version ${manifest!.version})`);
  } else if (state === "stale") {
    deps.out(`daemon not running (stale manifest, pid ${manifest!.pid})`);
  } else {
    deps.out("daemon not running");
  }

  return running ? 0 : 1;
}

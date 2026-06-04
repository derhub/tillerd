import { parseArgs, type ParseArgsConfig } from "util";
import type { Logger } from "@athing/logger";
import type { SetupContext, SetupDefinition } from "@athing/sdk";

export interface ManifestData {
  pid: number;
  version: string;
}

/**
 * Everything the CLI touches, injected so the core is exercised against fixtures
 * and fakes — never the operator's real settings file or daemon manifest.
 */
export interface CliDeps {
  setup: SetupDefinition;
  buildContext(notifyCommand: string, logger: Logger): SetupContext;
  resolveNotify(): string;
  readManifest(): ManifestData | null;
  isAlive(pid: number): boolean;
  isTTY: boolean;
  confirm(message: string): Promise<boolean>;
  out(line: string): void;
  err(line: string): void;
}

export const USAGE = `athing — controller/installer

Usage:
  athing install [--yes]    Install agent hooks into the agent settings file
  athing uninstall          Remove the hooks this tool installed
  athing status [--json]    Report whether the daemon is running

Flags:
  --yes         Skip interactive confirmation (also implied when not a TTY)
  --json        Machine-readable output (status only)
  -h, --help    Show this help`;

interface LogRecord {
  msg: string;
  extra?: Record<string, unknown>;
}

function recordingLogger(sink: LogRecord[]): Logger {
  const rec = () => (msg: string, extra?: Record<string, unknown>) => {
    sink.push({ msg, extra });
  };
  const logger: Logger = {
    debug: rec(),
    info: rec(),
    warn: rec(),
    error: rec(),
    // Setup recording is context-agnostic; child writes to the same sink.
    child: () => logger,
  };
  return logger;
}

type OptionSpec = ParseArgsConfig["options"];

const COMMAND_OPTIONS: Record<string, OptionSpec> = {
  install: { yes: { type: "boolean", default: false } },
  uninstall: {},
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

  switch (sub) {
    case "install":
      return install(deps, Boolean(values["yes"]));
    case "uninstall":
      return uninstall(deps);
    default:
      return status(deps, Boolean(values["json"]));
  }
}

async function install(deps: CliDeps, yes: boolean): Promise<number> {
  if (deps.isTTY && !yes) {
    const ok = await deps.confirm("Install agent hooks into the agent settings file?");
    if (!ok) {
      deps.err("install cancelled");
      return 1;
    }
  }

  let notify: string;
  try {
    notify = deps.resolveNotify();
  } catch (e) {
    deps.err(`install failed: ${(e as Error).message}`);
    return 1;
  }

  const records: LogRecord[] = [];
  await deps.setup.install(deps.buildContext(notify, recordingLogger(records)));

  const installed = records.find((r) => r.msg === "hooks installed");
  if (installed) {
    const events = (installed.extra?.["events"] as string[] | undefined) ?? [];
    deps.out(`installed hooks: ${events.join(", ")}`);
  } else {
    deps.out("hooks already installed");
  }
  return 0;
}

async function uninstall(deps: CliDeps): Promise<number> {
  const records: LogRecord[] = [];
  await deps.setup.uninstall(deps.buildContext("", recordingLogger(records)));

  const removed = records.find((r) => r.msg === "hooks uninstalled");
  deps.out(removed ? "hooks uninstalled" : "nothing to remove");
  return 0;
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

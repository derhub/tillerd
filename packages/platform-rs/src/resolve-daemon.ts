import * as fs from "node:fs";
import { join, resolve } from "node:path";
import { homedir } from "node:os";
import { AtError } from "@athing/sdk";

const BUILD_OUTPUT = join(import.meta.dir, "../../daemon-rs/target/release/athing-daemon");
const BUILD_STEP = "cargo build --release in packages/daemon-rs";

/** Probes the native resolver reads; injectable so resolution order is testable. */
export interface DaemonResolveProbes {
  env?: Record<string, string | undefined>;
  exists?: (path: string) => boolean;
  cwd?: string;
  home?: string;
}

/**
 * Resolve the native daemon binary: an explicit override, then the native build
 * output, then established install locations. Unlike the reference resolver this
 * deliberately avoids a generic PATH name lookup, which would silently select the
 * reference daemon instead of the native build.
 */
export function resolveNativeDaemonBinary(probes: DaemonResolveProbes = {}): string {
  const env = probes.env ?? process.env;
  const exists = probes.exists ?? fs.existsSync;
  const cwd = probes.cwd ?? process.cwd();
  const home = probes.home ?? homedir();

  const envBin = env["ATHING_DAEMON_BIN"];
  if (envBin) {
    const abs = resolve(envBin);
    if (exists(abs)) return abs;
  }

  if (exists(BUILD_OUTPUT)) return BUILD_OUTPUT;

  const installLocations = [
    join(cwd, "bin", "athing-daemon"),
    join(home, ".local", "bin", "athing-daemon"),
  ];
  for (const loc of installLocations) {
    if (exists(loc)) return loc;
  }

  throw new AtError(
    "BinaryNotFound",
    `Cannot resolve the native athing-daemon. Set ATHING_DAEMON_BIN or run \`${BUILD_STEP}\`.`,
  );
}

/** The native build-output path, exported for tests and diagnostics. */
export const NATIVE_BUILD_OUTPUT = BUILD_OUTPUT;

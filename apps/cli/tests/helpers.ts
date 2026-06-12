import type { CliDeps, ManifestData } from "../src/cli";

export interface HarnessOverrides {
  manifest?: ManifestData | null;
  isAlive?: boolean;
}

export interface Harness {
  deps: CliDeps;
  out: string[];
  err: string[];
}

export function harness(o: HarnessOverrides = {}): Harness {
  const out: string[] = [];
  const err: string[] = [];

  const deps: CliDeps = {
    readManifest: () => o.manifest ?? null,
    isAlive: () => o.isAlive ?? false,
    out: (line) => out.push(line),
    err: (line) => err.push(line),
  };

  return { deps, out, err };
}

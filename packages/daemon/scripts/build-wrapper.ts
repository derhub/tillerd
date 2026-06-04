// Builds the daemon for Node: bundles the TS sources (resolving workspace
// imports) into dist/daemon.mjs with node-pty kept external (native addon
// resolved at runtime), then writes bin/athing-daemon as a portable shell
// wrapper that runs it via `node`. The daemon runs on Node — node-pty's fd IO
// is broken under Bun.
import { chmodSync, mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const pkgDir = resolve(import.meta.dir, "..");
const root = resolve(pkgDir, "../..");
const bundle = resolve(pkgDir, "dist/daemon.mjs");

mkdirSync(resolve(pkgDir, "dist"), { recursive: true });

const build = await Bun.build({
  entrypoints: [resolve(pkgDir, "src/main.ts")],
  target: "node",
  external: ["node-pty"],
  outdir: resolve(pkgDir, "dist"),
  naming: "daemon.mjs",
});
if (!build.success) {
  for (const log of build.logs) console.error(log);
  throw new Error("daemon bundle failed");
}

mkdirSync(resolve(root, "bin"), { recursive: true });
const outFile = resolve(root, "bin/athing-daemon");
const wrapper = [
  "#!/bin/sh",
  // $0 = bin/athing-daemon; parent dir = project root.
  'ROOT="$(cd "$(dirname "$0")/.." && pwd)"',
  'exec node "$ROOT/packages/daemon/dist/daemon.mjs" "$@"',
  "",
].join("\n");

writeFileSync(outFile, wrapper, "utf8");
chmodSync(outFile, 0o755);

console.log("wrote", bundle, "and", outFile);

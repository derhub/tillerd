// Generates bin/athing-daemon as a portable shell wrapper that invokes the
// daemon via bun, avoiding posix_spawnp issues inside a compiled Bun binary.
// Paths are relative to the wrapper's own location so the repo stays portable.
import { writeFileSync, chmodSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";

const outFile = resolve(import.meta.dir, "../../../bin/athing-daemon");

mkdirSync(resolve(import.meta.dir, "../../../bin"), { recursive: true });

// $0 = bin/athing-daemon; parent dir = project root.
const wrapper = [
  "#!/bin/sh",
  'ROOT="$(cd "$(dirname "$0")/.." && pwd)"',
  'exec bun "$ROOT/packages/daemon/src/main.ts" "$@"',
  "",
].join("\n");

writeFileSync(outFile, wrapper, "utf8");
chmodSync(outFile, 0o755);

console.log("wrote", outFile);

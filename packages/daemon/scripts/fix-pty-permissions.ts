// node-pty's spawn-helper must be executable or posix_spawnp fails silently.
import { chmodSync, existsSync } from "node:fs";
import { resolve } from "node:path";

const candidates = [
  resolve(import.meta.dir, "../node_modules/node-pty/prebuilds/darwin-arm64/spawn-helper"),
  resolve(import.meta.dir, "../node_modules/node-pty/prebuilds/darwin-x64/spawn-helper"),
  resolve(import.meta.dir, "../node_modules/node-pty/prebuilds/linux-x64/spawn-helper"),
  resolve(import.meta.dir, "../node_modules/node-pty/prebuilds/linux-arm64/spawn-helper"),
  resolve(import.meta.dir, "../node_modules/node-pty/build/Release/spawn-helper"),
];

for (const p of candidates) {
  if (existsSync(p)) {
    chmodSync(p, 0o755);
    console.log("chmod +x", p);
  }
}

#!/usr/bin/env bun
import { run, type CliDeps } from "./cli";

const deps: CliDeps = {
  readManifest: () => null,
  isAlive: () => false,
  out: (line) => console.log(line),
  err: (line) => console.error(line),
};

run(process.argv.slice(2), deps).then(
  (code) => process.exit(code),
  (err: unknown) => {
    console.error(err);
    process.exit(1);
  },
);

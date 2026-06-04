// Enforces runtime neutrality: @athing/sdk and @athing/engine must use Web APIs
// only — never Node/Bun globals. Run in check/CI.
import { Glob } from "bun";

const ROOTS = ["packages/sdk/src", "packages/engine/src"];
const FORBIDDEN: Array<{ re: RegExp; label: string }> = [
  { re: /\bBuffer\b/, label: "Buffer" },
  { re: /\bnode:/, label: "node: import" },
  { re: /\bBun\./, label: "Bun.*" },
  // Negative lookbehind for a quote so OTel key literals/property names like
  // "process.pid" are not mistaken for the Node `process` global.
  { re: /(?<!["'])\bprocess\./, label: "process.*" },
  { re: /\brequire\(/, label: "require(" },
];

const violations: string[] = [];

for (const root of ROOTS) {
  const glob = new Glob("**/*.ts");
  for await (const rel of glob.scan({ cwd: root })) {
    const path = `${root}/${rel}`;
    const text = await Bun.file(path).text();
    const lines = text.split("\n");
    lines.forEach((line, i) => {
      for (const { re, label } of FORBIDDEN) {
        if (re.test(line)) violations.push(`${path}:${i + 1}: ${label} → ${line.trim()}`);
      }
    });
  }
}

if (violations.length > 0) {
  console.error("Web-API gate FAILED — Node/Bun globals in neutral layers:");
  for (const v of violations) console.error("  " + v);
  process.exit(1);
}
console.log("Web-API gate PASS — sdk + engine are runtime-neutral.");

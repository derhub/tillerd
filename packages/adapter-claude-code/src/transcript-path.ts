import * as path from "node:path";
import * as os from "node:os";

export function encodeCwd(cwd: string): string {
  return cwd.replace(/\//g, "-").replace(/^-/, "");
}

export function transcriptPath(sessionId: string, cwd: string): string {
  const encoded = encodeCwd(path.resolve(cwd));
  return path.join(os.homedir(), ".claude", "projects", encoded, `${sessionId}.jsonl`);
}

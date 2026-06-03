export function encodeCwd(cwd: string): string {
  return cwd.replace(/\//g, "-").replace(/^-/, "");
}

export function transcriptPath(sessionId: string, cwd: string, agentHome: string): string {
  const encoded = encodeCwd(cwd);
  return `${agentHome}/projects/${encoded}/${sessionId}.jsonl`;
}

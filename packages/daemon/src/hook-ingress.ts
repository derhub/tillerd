import type { Logger } from "@athing/logger";
import * as fs from "node:fs";
import * as http from "node:http";
import { timingSafeEqual } from "node:crypto";

function tokensMatch(a: string, b: string): boolean {
  const ab = Buffer.from(a, "utf8");
  const bb = Buffer.from(b, "utf8");
  if (ab.length !== bb.length) return false;
  return timingSafeEqual(ab, bb);
}

interface HookIngressOptions {
  socketPath: string;
  getToken: (sessionId: string) => string | null;
  onHook: (sessionId: string, payload: unknown) => void;
  logger: Logger;
}

type Outcome = { status: number; body: string };

export class HookIngress {
  private nodeServer: http.Server | null = null;
  private processed = new Set<string>();

  constructor(private readonly opts: HookIngressOptions) {}

  // Authenticate, dedup, dispatch.
  private handle(method: string, sessionId: string, token: string, bodyText: string): Outcome {
    const { getToken, onHook, logger } = this.opts;
    if (method !== "POST") return { status: 405, body: "method not allowed" };
    if (!sessionId || !token) {
      logger.warn("hook-ingress: missing headers");
      return { status: 401, body: "unauthorized" };
    }
    const expected = getToken(sessionId);
    if (expected === null) return { status: 403, body: "forbidden" };
    if (!tokensMatch(token, expected)) {
      logger.warn("hook-ingress: token mismatch", { sessionId });
      return { status: 403, body: "forbidden" };
    }
    let payload: unknown;
    try {
      payload = JSON.parse(bodyText);
    } catch (err) {
      logger.warn("hook-ingress: parse error", { err: String(err) });
      return { status: 400, body: "bad request" };
    }
    const key = `${sessionId}:${JSON.stringify(payload)}`;
    if (this.processed.has(key)) return { status: 200, body: "ok" };
    this.processed.add(key);
    pruneProcessed(this.processed);
    onHook(sessionId, payload);
    return { status: 200, body: "ok" };
  }

  start(): void {
    if (this.nodeServer) return;
    const { logger } = this.opts;

    this.nodeServer = http.createServer((req, res) => {
      const chunks: Buffer[] = [];
      req.on("data", (c: Buffer) => chunks.push(c));
      req.on("end", () => {
        const hdr = (n: string) =>
          (Array.isArray(req.headers[n]) ? req.headers[n]![0] : (req.headers[n] as string)) ?? "";
        const { status, body } = this.handle(
          req.method ?? "",
          hdr("x-session-id"),
          hdr("x-session-token"),
          Buffer.concat(chunks).toString("utf8"),
        );
        res.writeHead(status);
        res.end(body);
      });
      req.on("error", () => {
        res.writeHead(400);
        res.end("bad request");
      });
    });
    this.nodeServer.listen(this.opts.socketPath);

    // Restrict the control-plane socket to the owner — defence in depth on top
    // of the per-session token (ADR-0007 authenticated control plane).
    try {
      fs.chmodSync(this.opts.socketPath, 0o600);
    } catch (err) {
      logger.warn("hook-ingress: chmod failed", { err: String(err) });
    }

    logger.info("hook ingress started", { sock: this.opts.socketPath });
  }

  stop(): void {
    this.nodeServer?.close();
    this.nodeServer = null;
  }
}

function pruneProcessed(set: Set<string>): void {
  if (set.size > 10_000) {
    const iter = set.values();
    for (let i = 0; i < 1_000; i++) {
      const val = iter.next().value;
      if (val !== undefined) set.delete(val);
    }
  }
}

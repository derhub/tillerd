import type { Logger } from "@athing/logger";
import * as fs from "node:fs";
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

export class HookIngress {
  private server: ReturnType<typeof Bun.serve> | null = null;
  private processed = new Set<string>();

  constructor(private readonly opts: HookIngressOptions) {}

  start(): void {
    if (this.server) return;

    const { getToken, onHook, logger } = this.opts;
    const processed = this.processed;

    this.server = Bun.serve({
      unix: this.opts.socketPath,
      fetch(req) {
        if (req.method !== "POST") {
          return new Response("method not allowed", { status: 405 });
        }

        const sessionId = req.headers.get("x-session-id") ?? "";
        const token = req.headers.get("x-session-token") ?? "";

        if (!sessionId || !token) {
          logger.warn("hook-ingress: missing headers");
          return new Response("unauthorized", { status: 401 });
        }

        const expected = getToken(sessionId);
        if (expected === null) {
          logger.debug("hook-ingress: unknown session", { sessionId });
          return new Response("forbidden", { status: 403 });
        }

        if (!tokensMatch(token, expected)) {
          logger.warn("hook-ingress: token mismatch", { sessionId });
          return new Response("forbidden", { status: 403 });
        }

        return req
          .json()
          .then((payload: unknown) => {
            const key = `${sessionId}:${JSON.stringify(payload)}`;
            if (processed.has(key)) {
              return new Response("ok", { status: 200 });
            }
            processed.add(key);
            pruneProcessed(processed);

            onHook(sessionId, payload);
            return new Response("ok", { status: 200 });
          })
          .catch((err: unknown) => {
            logger.warn("hook-ingress: parse error", { err: String(err) });
            return new Response("bad request", { status: 400 });
          });
      },
    });

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
    this.server?.stop(true);
    this.server = null;
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

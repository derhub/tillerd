import type { Logger } from "../logger";
import type { HookDispatcher } from "./dispatcher";

export class HookReceiver {
  private server: ReturnType<typeof Bun.serve> | null = null;
  private _url = "";

  constructor(
    private readonly dispatcher: HookDispatcher,
    private readonly logger: Logger,
  ) {}

  get url(): string {
    return this._url;
  }

  start(): void {
    if (this.server) return;

    const dispatcher = this.dispatcher;
    const logger = this.logger;

    this.server = Bun.serve({
      port: 0,
      hostname: "127.0.0.1",
      fetch(req) {
        if (req.method !== "POST") {
          return new Response("method not allowed", { status: 405 });
        }

        const sessionId = req.headers.get("x-session-id") ?? "";
        const token = req.headers.get("x-session-token") ?? "";

        if (!sessionId || !token) {
          logger.warn("hook.receiver: missing headers");
          return new Response("unauthorized", { status: 401 });
        }

        return req
          .json()
          .then((payload: unknown) => {
            const ok = dispatcher.dispatch(sessionId, token, payload);
            if (!ok) return new Response("rejected", { status: 403 });
            return new Response("ok", { status: 200 });
          })
          .catch((err: unknown) => {
            logger.warn("hook.receiver: parse error", { err: String(err) });
            return new Response("bad request", { status: 400 });
          });
      },
    });

    this._url = `http://127.0.0.1:${this.server.port}`;
    logger.info("hook receiver started", { url: this._url });
  }

  stop(): void {
    this.server?.stop(true);
    this.server = null;
    this._url = "";
  }
}

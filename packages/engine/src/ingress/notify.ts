const MAX_PAYLOAD_BYTES = 16 * 1024 * 1024;
const chunks: Buffer[] = [];
let collected = 0;
let overflow = false;
process.stdin.on("data", (c: Buffer) => {
  collected += c.length;
  if (collected > MAX_PAYLOAD_BYTES) {
    overflow = true;
    return;
  }
  chunks.push(c);
});
process.stdin.on("end", async () => {
  if (overflow) return;
  const raw = Buffer.concat(chunks).toString("utf8");
  const bridgeUrl = process.env["ATHING_BRIDGE_URL"];
  const token = process.env["ATHING_SESSION_TOKEN"];
  const sessionId = process.env["ATHING_SESSION_ID"];
  if (!bridgeUrl) return;
  try {
    const isSocket = bridgeUrl.startsWith("/");
    await fetch(isSocket ? "http://localhost" : bridgeUrl, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "x-session-token": token ?? "",
        "x-session-id": sessionId ?? "",
      },
      body: raw,
      ...(isSocket ? { unix: bridgeUrl } : {}),
    } as RequestInit);
  } catch {}
});

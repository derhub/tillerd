// packages/engine/src/ingress/notify.ts
var chunks = [];
process.stdin.on("data", (c) => chunks.push(c));
process.stdin.on("end", async () => {
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
    });
  } catch {}
});

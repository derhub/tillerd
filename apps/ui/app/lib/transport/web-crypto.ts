// crypto.randomUUID() / getRandomValues() require a secure context. The Tauri v2 custom-protocol
// webview qualifies; feature-detect with fallback so a non-secure host always has an id source.

export function hasSecureCrypto(): boolean {
  const c = globalThis.crypto as Crypto | undefined;
  return !!c && typeof c.randomUUID === "function";
}

export function randomId(): string {
  const c = globalThis.crypto as Crypto | undefined;
  if (c && typeof c.randomUUID === "function") return c.randomUUID();
  if (c && typeof c.getRandomValues === "function") {
    return uuidFromBytes(c.getRandomValues(new Uint8Array(16)));
  }
  return uuidFromBytes(weakRandomBytes(16));
}

function uuidFromBytes(bytes: Uint8Array): string {
  const b = bytes.slice(0, 16);
  b[6] = (b[6]! & 0x0f) | 0x40; // version 4
  b[8] = (b[8]! & 0x3f) | 0x80; // variant 10
  const hex = Array.from(b, (n) => n.toString(16).padStart(2, "0"));
  return `${hex.slice(0, 4).join("")}-${hex.slice(4, 6).join("")}-${hex
    .slice(6, 8)
    .join("")}-${hex.slice(8, 10).join("")}-${hex.slice(10, 16).join("")}`;
}

function weakRandomBytes(n: number): Uint8Array {
  const out = new Uint8Array(n);
  for (let i = 0; i < n; i++) out[i] = Math.floor(Math.random() * 256);
  return out;
}

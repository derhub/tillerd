export function hasSecureCrypto(): boolean {
  return typeof crypto !== "undefined" && typeof crypto.randomUUID === "function";
}

export function uuid(): string {
  return crypto.randomUUID();
}

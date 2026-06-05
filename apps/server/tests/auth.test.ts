import { test, expect, describe } from "bun:test";
import { isOriginAllowed, parseAllowedOrigins } from "../src/auth";

describe("isOriginAllowed", () => {
  const allowed = parseAllowedOrigins(undefined, 3000);

  test("allows a request with no Origin header (native client)", () => {
    expect(isOriginAllowed(null, allowed)).toBe(true);
  });

  test("allows the server's own loopback origin", () => {
    expect(isOriginAllowed("http://localhost:3000", allowed)).toBe(true);
  });

  test("rejects an untrusted cross-site origin", () => {
    expect(isOriginAllowed("https://evil.example", allowed)).toBe(false);
  });

  test("rejects the literal null origin string from a sandboxed page", () => {
    expect(isOriginAllowed("null", allowed)).toBe(false);
  });
});

describe("parseAllowedOrigins", () => {
  test("includes both loopback hosts for the server port", () => {
    const origins = parseAllowedOrigins(undefined, 8080);
    expect(origins.has("http://localhost:8080")).toBe(true);
    expect(origins.has("http://127.0.0.1:8080")).toBe(true);
  });

  test("adds trimmed comma-separated extras from the env value", () => {
    const origins = parseAllowedOrigins("https://app.example , http://localhost:5173", 3000);
    expect(origins.has("https://app.example")).toBe(true);
    expect(origins.has("http://localhost:5173")).toBe(true);
  });
});

import { test, expect } from "bun:test";
import { encodeKey, encodeKeySequence } from "./keys";

const bytes = (spec: string, opts?: { applicationCursor?: boolean }) =>
  Array.from(encodeKey(spec, opts));

test("named control keys encode to their canonical bytes", () => {
  expect(bytes("Enter")).toEqual([0x0d]);
  expect(bytes("Tab")).toEqual([0x09]);
  expect(bytes("Escape")).toEqual([0x1b]);
  expect(bytes("Backspace")).toEqual([0x7f]);
  expect(bytes("Space")).toEqual([0x20]);
});

test("function keys encode to their escape sequences", () => {
  expect(bytes("F1")).toEqual([0x1b, 0x4f, 0x50]);
  expect(bytes("F5")).toEqual([0x1b, 0x5b, 0x31, 0x35, 0x7e]);
  expect(bytes("F12")).toEqual([0x1b, 0x5b, 0x32, 0x34, 0x7e]);
});

test("arrow keys honor application-cursor mode", () => {
  expect(bytes("Up")).toEqual([0x1b, 0x5b, 0x41]);
  expect(bytes("Up", { applicationCursor: true })).toEqual([0x1b, 0x4f, 0x41]);
});

test("Ctrl+letter maps to the matching control byte", () => {
  expect(bytes("Ctrl+A")).toEqual([0x01]);
  expect(bytes("Ctrl+C")).toEqual([0x03]);
  expect(bytes("Ctrl+Z")).toEqual([0x1a]);
});

test("Ctrl symbol combos and Ctrl+Space map to their control bytes", () => {
  expect(bytes("Ctrl+[")).toEqual([0x1b]);
  expect(bytes("Ctrl+\\")).toEqual([0x1c]);
  expect(bytes("Ctrl+?")).toEqual([0x7f]);
  expect(bytes("Ctrl+Space")).toEqual([0x00]);
});

test("Alt prefixes the key with ESC", () => {
  expect(bytes("Alt+F")).toEqual([0x1b, 0x46]);
  expect(bytes("Alt+f")).toEqual([0x1b, 0x66]);
  expect(bytes("Alt+Enter")).toEqual([0x1b, 0x0d]);
});

test("Ctrl+Alt combines an ESC prefix with the control byte", () => {
  expect(bytes("Ctrl+Alt+C")).toEqual([0x1b, 0x03]);
});

test("Shift uppercases letters", () => {
  expect(bytes("Shift+a")).toEqual([0x41]);
});

test("a bare character passes through as UTF-8", () => {
  expect(bytes(":")).toEqual([0x3a]);
  expect(Array.from(encodeKey("é"))).toEqual([0xc3, 0xa9]);
});

test("encodeKeySequence returns one buffer per space-separated key", () => {
  const seq = encodeKeySequence("Escape : w q Enter");
  expect(seq.length).toBe(5);
  expect(Array.from(seq[0] as Uint8Array)).toEqual([0x1b]);
  expect(Array.from(seq[4] as Uint8Array)).toEqual([0x0d]);
});

test("encodeKey concatenates a sequence with no inter-key delay", () => {
  expect(bytes("Ctrl+X m")).toEqual([0x18, 0x6d]);
});

test("unrecognized keys throw", () => {
  expect(() => encodeKey("Frobnicate")).toThrow(/unrecognized key/);
  expect(() => encodeKey("   ")).toThrow(/empty key spec/);
});

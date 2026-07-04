import { expect, test } from "bun:test";

import { classifyTerminalKey, linkModifierHeld } from "./keymap";

function key(over: Partial<Parameters<typeof classifyTerminalKey>[0]>) {
  return { key: "a", ctrlKey: false, metaKey: false, shiftKey: false, ...over };
}

test("mac: Cmd+F opens find", () => {
  expect(classifyTerminalKey(key({ key: "f", metaKey: true }), true)).toBe("find");
});

test("mac: Cmd+C copies and Cmd+V pastes", () => {
  expect(classifyTerminalKey(key({ key: "c", metaKey: true }), true)).toBe("copy");
  expect(classifyTerminalKey(key({ key: "v", metaKey: true }), true)).toBe("paste");
});

test("linux: Ctrl+F opens find", () => {
  expect(classifyTerminalKey(key({ key: "f", ctrlKey: true }), false)).toBe("find");
});

test("linux: Ctrl+Shift+C copies and Ctrl+Shift+V pastes", () => {
  expect(classifyTerminalKey(key({ key: "c", ctrlKey: true, shiftKey: true }), false)).toBe("copy");
  expect(classifyTerminalKey(key({ key: "v", ctrlKey: true, shiftKey: true }), false)).toBe(
    "paste",
  );
});

test("linux: bare Ctrl+C is left to the PTY as SIGINT", () => {
  expect(classifyTerminalKey(key({ key: "c", ctrlKey: true }), false)).toBeNull();
});

test("mac: Ctrl+C is left to the PTY as SIGINT", () => {
  expect(classifyTerminalKey(key({ key: "c", ctrlKey: true }), true)).toBeNull();
});

test("an ordinary keystroke is not intercepted", () => {
  expect(classifyTerminalKey(key({ key: "a" }), true)).toBeNull();
  expect(classifyTerminalKey(key({ key: "a" }), false)).toBeNull();
});

test("link activation requires Cmd on macOS and Ctrl elsewhere", () => {
  expect(linkModifierHeld({ metaKey: true, ctrlKey: false }, true)).toBe(true);
  expect(linkModifierHeld({ metaKey: false, ctrlKey: false }, true)).toBe(false);
  expect(linkModifierHeld({ metaKey: false, ctrlKey: true }, false)).toBe(true);
  expect(linkModifierHeld({ metaKey: false, ctrlKey: false }, false)).toBe(false);
});

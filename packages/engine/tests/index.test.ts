import { test, expect } from "bun:test";
import { Engine } from "../src/index";

test("Engine initializes with adapter", () => {
  const adapter = {
    name: "test",
    description: "test adapter",
  };

  const engine = new Engine(adapter);
  expect(engine).toBeDefined();
});

test("Engine.parseHook returns status", () => {
  const adapter = {
    name: "test",
    description: "test adapter",
  };

  const engine = new Engine(adapter);
  const status = engine.parseHook({
    type: "test",
    timestamp: Date.now(),
    data: null,
  });

  expect(status).toBe("IDLE");
});

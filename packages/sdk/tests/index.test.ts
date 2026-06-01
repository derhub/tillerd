import { test, expect } from "bun:test";
import type { SessionStatus, HookEvent, AgentDefinition } from "../src/index";

test("SessionStatus type is correct", () => {
  const status: SessionStatus = "IDLE";
  expect(status).toBe("IDLE");
});

test("HookEvent interface is valid", () => {
  const event: HookEvent = {
    type: "test",
    timestamp: Date.now(),
    data: { foo: "bar" },
  };
  expect(event.type).toBe("test");
  expect(typeof event.timestamp).toBe("number");
});

test("AgentDefinition interface is valid", () => {
  const agent: AgentDefinition = {
    name: "test-agent",
    description: "A test agent",
  };
  expect(agent.name).toBe("test-agent");
  expect(agent.description).toBe("A test agent");
});

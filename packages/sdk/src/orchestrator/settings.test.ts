import { test, expect } from "bun:test";
import {
  createSettingsClient,
  SETTING_GET,
  SETTING_SET,
  SETTING_LIST,
  type SettingsTransport,
  type SettingEntry,
} from "./settings";

function fakeTransport(result: unknown = null) {
  const calls: { command: string; args?: Record<string, unknown> }[] = [];
  const transport: SettingsTransport = {
    invoke: async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      return result as T;
    },
  };
  return { transport, calls };
}

test("setSetting then getSetting round-trips a value through the transport", async () => {
  const store = new Map<string, unknown>();
  const transport: SettingsTransport = {
    invoke: async <T>(command: string, args?: Record<string, unknown>) => {
      if (command === SETTING_SET) {
        store.set(String(args?.key), args?.value);
        return undefined as T;
      }
      return (store.get(String(args?.key)) ?? null) as T;
    },
  };
  const client = createSettingsClient(transport);

  await client.setSetting({ scope: "global", key: "theme", value: "dark" });
  const value = await client.getSetting({ scope: "global", key: "theme" });

  expect(value).toBe("dark");
});

test("getSetting routes to the get command with scope, projectId, and key", async () => {
  const { transport, calls } = fakeTransport("light");
  const client = createSettingsClient(transport);

  const value = await client.getSetting({ scope: "project", projectId: "p1", key: "theme" });

  expect(calls).toEqual([
    { command: SETTING_GET, args: { scope: "project", projectId: "p1", key: "theme" } },
  ]);
  expect(value).toBe("light");
});

test("getSetting resolves to null when the key is unset", async () => {
  const { transport } = fakeTransport(null);
  const client = createSettingsClient(transport);

  const value = await client.getSetting({ scope: "global", key: "missing" });

  expect(value).toBeNull();
});

test("listSettings returns the typed entry list for a scope", async () => {
  const entries: SettingEntry[] = [
    { key: "a", value: 1 },
    { key: "b", value: "two" },
  ];
  const { transport, calls } = fakeTransport(entries);
  const client = createSettingsClient(transport);

  const result = await client.listSettings({ scope: "global" });

  expect(calls).toEqual([{ command: SETTING_LIST, args: { scope: "global" } }]);
  expect(result).toEqual(entries);
});

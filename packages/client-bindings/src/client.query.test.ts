import { describe, expect, test } from "bun:test";

import { command, entityKey, query } from "./client.query";
import { commands } from "./tauri_bindings.gen";

const NAMES = Object.keys(commands) as Array<keyof typeof commands>;

describe("entityKey coverage", () => {
  // Unclassified command -> build failure here, not a silent undefined key.
  test("every generated command classifies to a non-empty cache key", () => {
    const unclassified = NAMES.filter((name) => {
      try {
        return !entityKey(name);
      } catch {
        return true;
      }
    });
    expect(unclassified).toEqual([]);
  });
});

describe("entityKey overlap classification", () => {
  // Order-sensitive (longest prefix first); coverage passes on a mis-bucket, so lock the overlaps.
  test.each([
    ["commandCenterSetLeader", "commandCenter"],
    ["commandRename", "commands"],
    ["settingsResolve", "settings"],
    ["settingList", "settings"],
    ["notificationsList", "notifications"],
    ["notificationMarkRead", "notifications"],
    ["launchTemplateCreate", "launchTemplates"],
    ["templateList", "templates"],
    ["sessionRename", "sessions"],
    ["surfaceChannel", "surfaces"],
  ] as const)("%s -> %s", (name, expected) => {
    expect(entityKey(name)).toBe(expected);
  });
});

describe("query key derivation", () => {
  test("query and command share the same entity key for an entity", () => {
    const q = query("sessionList", { projectId: null, limit: null, offset: null });
    const c = command("sessionRename");
    expect(q.queryKey[0]).toBe("sessions");
    expect(c.meta?.invalidates).toEqual([["sessions"]]);
  });

  test("query key carries entity, verb, and args", () => {
    const q = query("sessionGet", { id: "s1" });
    expect([...q.queryKey]).toEqual(["sessions", "get", { id: "s1" }]);
  });
});

describe("invalidation", () => {
  test("default mutation invalidates only its own entity", () => {
    expect(command("workspaceRename").meta?.invalidates).toEqual([["workspaces"]]);
  });

  test("CROSS cascade invalidates multiple entities", () => {
    expect(command("projectArchive").meta?.invalidates).toEqual([["projects"], ["sessions"]]);
  });
});

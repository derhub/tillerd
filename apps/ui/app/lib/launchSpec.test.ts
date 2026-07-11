import { describe, expect, test } from "bun:test";

import {
  CURRENT_SPEC_VERSION,
  describeLaunchSpec,
  emptySpec,
  parseLaunchSpec,
  serializeLaunchSpec,
  validateSpec,
  type LaunchSpec,
} from "./launchSpec";

// A real spec with a placed item and an unplaced item -- the fixture IS the wire
// contract the orchestrator's launch_spec.rs parses.
const REAL_SPEC =
  '{"version":1,"items":[' +
  '{"target":"terminal","placement":"p-1","command":{"library_ref":"login-shell"}},' +
  '{"target":"terminal","command":{"library_ref":"htop"}}' +
  "]}";

describe("launch spec round-trip", () => {
  test("parse then serialize preserves the spec unchanged", () => {
    const serialized = serializeLaunchSpec(parseLaunchSpec(REAL_SPEC));
    expect(JSON.parse(serialized)).toEqual(JSON.parse(REAL_SPEC));
  });

  test("an inline command ref survives an edit of a sibling item", () => {
    const input =
      '{"version":1,"items":[' +
      '{"target":"terminal","command":{"executable":"/bin/bash","args":["-l"]}},' +
      '{"target":"terminal","command":{"library_ref":"htop"}}' +
      "]}";
    const spec = parseLaunchSpec(input);
    // Edit only the second (library) item's ref; the inline item must be untouched.
    const edited: LaunchSpec = {
      ...spec,
      items: [spec.items[0], { ...spec.items[1], command: { library_ref: "btop" } }],
    };
    const out = JSON.parse(serializeLaunchSpec(edited));
    expect(out.items[0]).toEqual({
      target: "terminal",
      command: { executable: "/bin/bash", args: ["-l"] },
    });
    expect(out.items[1].command).toEqual({ library_ref: "btop" });
  });

  test("placement is omitted, never emitted as null, when absent", () => {
    const out = serializeLaunchSpec(
      parseLaunchSpec(
        '{"version":1,"items":[{"target":"terminal","command":{"library_ref":"x"}}]}',
      ),
    );
    expect(out).not.toContain("placement");
  });
});

describe("launch spec parse guards", () => {
  test("a missing version is rejected", () => {
    expect(() => parseLaunchSpec('{"items":[]}')).toThrow();
  });

  test("a zero version is rejected", () => {
    expect(() => parseLaunchSpec('{"version":0,"items":[]}')).toThrow();
  });

  test("an empty item list is valid", () => {
    expect(parseLaunchSpec('{"version":1,"items":[]}').items).toEqual([]);
  });
});

describe("validateSpec", () => {
  test("an item with no command selected is an error", () => {
    const spec: LaunchSpec = {
      version: CURRENT_SPEC_VERSION,
      items: [{ target: "terminal", command: { library_ref: "" } }],
    };
    expect(validateSpec(spec)).toHaveLength(1);
  });

  test("a fully-picked spec has no errors", () => {
    const spec: LaunchSpec = {
      version: CURRENT_SPEC_VERSION,
      items: [{ target: "terminal", command: { library_ref: "login-shell" } }],
    };
    expect(validateSpec(spec)).toEqual([]);
  });

  test("an empty spec has no errors", () => {
    expect(validateSpec(emptySpec())).toEqual([]);
  });
});

describe("describeLaunchSpec", () => {
  test("labels a library-ref item by its resolved command name", () => {
    const label = describeLaunchSpec(
      '{"version":1,"items":[{"target":"terminal","command":{"library_ref":"htop-id"}}]}',
      (id) => (id === "htop-id" ? "htop" : undefined),
    );
    expect(label).toBe("htop");
  });

  test("labels an inline item by its executable", () => {
    const label = describeLaunchSpec(
      '{"version":1,"items":[{"target":"terminal","command":{"executable":"/bin/bash","args":[]}}]}',
      () => undefined,
    );
    expect(label).toBe("/bin/bash");
  });

  test("an unresolvable library ref reads as unknown", () => {
    const label = describeLaunchSpec(
      '{"version":1,"items":[{"target":"terminal","command":{"library_ref":"gone"}}]}',
      () => undefined,
    );
    expect(label).toBe("(unknown command)");
  });

  test("an empty item list reads as empty", () => {
    expect(describeLaunchSpec('{"version":1,"items":[]}', () => undefined)).toBe(
      "(empty template)",
    );
  });

  test("invalid JSON reads as invalid", () => {
    expect(describeLaunchSpec("not json", () => undefined)).toBe("(invalid spec)");
  });
});

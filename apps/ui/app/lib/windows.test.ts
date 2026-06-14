import { test, expect, describe } from "bun:test";
import {
  detachedLabel,
  projectLabel,
  detachedQuery,
  projectQuery,
  parseWindowIntent,
} from "./windows";

describe("window labels", () => {
  test("detached label namespaces the placement", () => {
    expect(detachedLabel("p1")).toBe("detached-p1");
  });

  test("project label namespaces the project id", () => {
    expect(projectLabel("pr1")).toBe("project-pr1");
  });
});

describe("parseWindowIntent", () => {
  test("no query is the main window", () => {
    expect(parseWindowIntent("")).toEqual({ kind: "main" });
  });

  test("detached intent carries session and placement", () => {
    expect(parseWindowIntent("?w=detached&session=s1&placement=p1")).toEqual({
      kind: "detached",
      sessionId: "s1",
      placement: "p1",
    });
  });

  test("detached intent missing placement falls back to main", () => {
    expect(parseWindowIntent("?w=detached&session=s1")).toEqual({ kind: "main" });
  });

  test("project intent carries project and optional session", () => {
    expect(parseWindowIntent("?w=project&project=pr1&session=s2")).toEqual({
      kind: "project",
      projectId: "pr1",
      sessionId: "s2",
    });
  });

  test("project intent without a session has a null session", () => {
    expect(parseWindowIntent("?w=project&project=pr1")).toEqual({
      kind: "project",
      projectId: "pr1",
      sessionId: null,
    });
  });
});

describe("intent queries round-trip through parseWindowIntent", () => {
  test("detached", () => {
    expect(parseWindowIntent(detachedQuery("s9", "p9"))).toEqual({
      kind: "detached",
      sessionId: "s9",
      placement: "p9",
    });
  });

  test("project with a session", () => {
    expect(parseWindowIntent(projectQuery("pr9", "s9"))).toEqual({
      kind: "project",
      projectId: "pr9",
      sessionId: "s9",
    });
  });

  test("project without a session", () => {
    expect(parseWindowIntent(projectQuery("pr9", null))).toEqual({
      kind: "project",
      projectId: "pr9",
      sessionId: null,
    });
  });
});

/// <reference lib="dom" />
import { test, expect, describe } from "bun:test";

import { parseWindowIntent } from "./windows";

describe("validateSearch / window intent", () => {
  test("detached intent resolves with sessionId and placement", () => {
    const intent = parseWindowIntent("?w=detached&session=s1&placement=p1");
    expect(intent).toEqual({ kind: "detached", sessionId: "s1", placement: "p1" });
  });

  test("project intent resolves with projectId", () => {
    const intent = parseWindowIntent("?w=project&project=pr1&session=s2");
    expect(intent).toEqual({ kind: "project", projectId: "pr1", sessionId: "s2" });
  });

  test("project intent without a session has null sessionId", () => {
    const intent = parseWindowIntent("?w=project&project=pr1");
    expect(intent).toEqual({ kind: "project", projectId: "pr1", sessionId: null });
  });

  test("missing intent falls back to main", () => {
    expect(parseWindowIntent("")).toEqual({ kind: "main" });
  });

  test("malformed w param falls back to main", () => {
    expect(parseWindowIntent("?w=unknown")).toEqual({ kind: "main" });
  });

  test("detached missing placement falls back to main", () => {
    expect(parseWindowIntent("?w=detached&session=s1")).toEqual({ kind: "main" });
  });

  test("workspace intent resolves", () => {
    expect(parseWindowIntent("?w=workspace&workspace=ws1")).toEqual({
      kind: "workspace",
      workspaceId: "ws1",
    });
  });

  test("workspace missing workspaceId falls back to main", () => {
    expect(parseWindowIntent("?w=workspace")).toEqual({ kind: "main" });
  });
});

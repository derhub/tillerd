import type { SettingView, TemplateView } from "@tillerd/client-bindings";

import { describe, expect, test } from "bun:test";

import { DEFAULT_TEMPLATE_KEY } from "~/lib/settings/keys";

import { librarySpecFor, resolveDefaultTemplate } from "./newSessionTemplate";

function template(id: string, name = id, specJson = '{"version":1,"items":[]}'): TemplateView {
  return { id, name, origin: "custom", pinned: false, specVersion: 1, specJson };
}

describe("resolveDefaultTemplate", () => {
  test("no settings resolves to no default", () => {
    expect(resolveDefaultTemplate(undefined)).toBeNull();
    expect(resolveDefaultTemplate([])).toBeNull();
  });

  test("a launch-template default resolves to a launch selection", () => {
    const settings: SettingView[] = [
      { key: DEFAULT_TEMPLATE_KEY, value: { kind: "launch", id: "lt-1" } },
    ];
    expect(resolveDefaultTemplate(settings)).toEqual({ kind: "launch", id: "lt-1" });
  });

  test("a library-template default resolves to a library selection", () => {
    const settings: SettingView[] = [
      { key: DEFAULT_TEMPLATE_KEY, value: { kind: "library", id: "tpl-1" } },
    ];
    expect(resolveDefaultTemplate(settings)).toEqual({ kind: "library", id: "tpl-1" });
  });

  test("a malformed default value is ignored", () => {
    const settings: SettingView[] = [{ key: DEFAULT_TEMPLATE_KEY, value: { kind: "launch" } }];
    expect(resolveDefaultTemplate(settings)).toBeNull();
  });

  test("an unrelated key is ignored", () => {
    const settings: SettingView[] = [{ key: "theme", value: "dark" }];
    expect(resolveDefaultTemplate(settings)).toBeNull();
  });
});

describe("librarySpecFor", () => {
  test("returns the matching template's spec", () => {
    const templates = [template("tpl-1", "One", '{"version":1,"items":[]}')];
    expect(librarySpecFor(templates, "tpl-1")).toEqual({
      specVersion: 1,
      specJson: '{"version":1,"items":[]}',
    });
  });

  test("returns null for an unknown id", () => {
    expect(librarySpecFor([template("tpl-1")], "missing")).toBeNull();
  });
});

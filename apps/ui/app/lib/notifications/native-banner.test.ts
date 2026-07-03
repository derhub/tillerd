import type { NotificationWire } from "@tillerd/client-bindings";

import { expect, test } from "bun:test";

import { raiseBanner, type BannerDeps } from "./native-banner";

function ev(over: Partial<NotificationWire> = {}): NotificationWire {
  return { id: "1", category: "surface-error", severity: "error", message: "boom", ts: 0, ...over };
}

function makeDeps(over: Partial<BannerDeps> = {}): {
  deps: BannerDeps;
  sent: Array<[string, string]>;
} {
  const sent: Array<[string, string]> = [];
  const deps: BannerDeps = {
    isFocused: async () => false,
    isPermissionGranted: async () => true,
    requestPermission: async () => true,
    send: (title, body) => sent.push([title, body]),
    ...over,
  };
  return { deps, sent };
}

test("raises a banner when unfocused and permission is granted", async () => {
  const { deps, sent } = makeDeps();
  await raiseBanner(ev({ title: "Terminal error" }), deps);
  expect(sent).toEqual([["Terminal error", "boom"]]);
});

test("raises no banner when the window is focused", async () => {
  const { deps, sent } = makeDeps({ isFocused: async () => true });
  await raiseBanner(ev(), deps);
  expect(sent).toHaveLength(0);
});

test("requests permission when not yet granted, then sends if allowed", async () => {
  let requested = false;
  const { deps, sent } = makeDeps({
    isPermissionGranted: async () => false,
    requestPermission: async () => {
      requested = true;
      return true;
    },
  });
  await raiseBanner(ev(), deps);
  expect(requested).toBe(true);
  expect(sent).toHaveLength(1);
});

test("raises no banner when permission is denied", async () => {
  const { deps, sent } = makeDeps({
    isPermissionGranted: async () => false,
    requestPermission: async () => false,
  });
  await raiseBanner(ev(), deps);
  expect(sent).toHaveLength(0);
});

import { expect, test } from "bun:test";

import { getApp } from "./shared-app";

// The setup preload owns the app lifecycle for both debug and bundled boot checks.
test("boots to a ready shell", async () => {
  expect(await getApp().$("body").getText()).toContain("services: ready");
}, 120_000);

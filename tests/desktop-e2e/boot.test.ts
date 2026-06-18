import { expect, test } from "bun:test";

import { getApp } from "./shared-app";

// The shared app's launch (setup.ts beforeAll) is the dev boot; assert it reached the ready shell.
test("boots to a ready shell", async () => {
  expect(await getApp().$("body").getText()).toContain("services: ready");
}, 120_000);

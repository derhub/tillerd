import { afterAll, beforeAll, beforeEach } from "bun:test";

import { resetToHome } from "./helpers";
import { getApp, launchSharedApp, teardownSharedApp } from "./shared-app";

// Hooks in a --preload module scope to the whole run: launch once, reset per test, reap once.
beforeAll(launchSharedApp);
afterAll(teardownSharedApp);
beforeEach(async () => {
  await resetToHome(getApp());
});

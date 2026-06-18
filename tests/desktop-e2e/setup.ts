import { afterAll, beforeAll, beforeEach } from "bun:test";

import { resetToHome } from "./helpers";
import { getApp, launchSharedApp, teardownSharedApp } from "./shared-app";

// Hooks in a --preload module scope to the whole run: launch once, reset per test, reap once.
// Explicit timeouts: a hook defaults to 5s, but a cold boot under software-GL xvfb exceeds that.
beforeAll(launchSharedApp, 120_000);
afterAll(teardownSharedApp, 30_000);
beforeEach(async () => {
  await resetToHome(getApp());
}, 30_000);

import { expect, test } from "bun:test";
import { SERVICE_HEALTH_METHOD, type ServiceHealth } from "@tillerd/sdk/orchestrator";

import { TauriServiceHealthSource, loadServiceHealthSource } from "./service-health-source";
import type { TauriCore } from "./tauri";

// Scenario: Desktop host provides the source
test("the desktop source snapshots through the service_health command", async () => {
  const health: ServiceHealth[] = [{ name: "tillerd-gate", version: "1.0.0", state: "ready" }];
  let called: string | undefined;
  const core = {
    invoke: async (cmd: string) => {
      called = cmd;
      return health;
    },
    createChannel: () => ({}),
  } as unknown as TauriCore;

  const result = await new TauriServiceHealthSource(core).snapshot();

  expect(called).toBe(SERVICE_HEALTH_METHOD);
  expect(result).toEqual(health);
});

// Scenario: Source absent on an unsupported host
test("the source is absent off the desktop host", async () => {
  expect(await loadServiceHealthSource()).toBeNull();
});

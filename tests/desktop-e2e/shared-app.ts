import { type Browser, launchReadyApp } from "./helpers";

// Module singleton shared across all spec files -- bun runs the suite in one process.
let app: Browser | undefined;

export async function launchSharedApp(): Promise<void> {
  app = await launchReadyApp();
}

export function getApp(): Browser {
  if (!app) {
    throw new Error("shared app not launched — run with `--preload ./tests/desktop-e2e/setup.ts`");
  }
  return app;
}

export async function teardownSharedApp(): Promise<void> {
  await app?.deleteSession();
  app = undefined;
}

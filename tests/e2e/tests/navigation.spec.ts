import { test, expect } from "@playwright/test";

// Renamed: was nav-link navigation. Now covers session routing.
test.describe("session routing", () => {
  test("navigating to /session/:id updates active sidebar row", async ({ page }) => {
    const SESSION_ID = "abc12345-0000-0000-0000-000000000000";
    await page.route("**/api/sessions", (route) =>
      route.fulfill({ json: { sessions: [{ id: SESSION_ID, cwd: "/tmp/test" }] } }),
    );
    // Block WS upgrade to avoid terminal init noise
    await page.route("**/ws/session**", (route) => route.abort());

    await page.goto(`/session/${SESSION_ID}`);
    await page.waitForLoadState("networkidle");

    const row = page.locator(`a[href='/session/${SESSION_ID}']`);
    await expect(row).toHaveClass(/bg-muted/);
  });

  test("/ route renders the spawning terminal pane", async ({ page }) => {
    await page.route("**/api/sessions", (route) => route.fulfill({ json: { sessions: [] } }));
    await page.route("**/ws/session**", (route) => route.abort());
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    // Terminal container inside the terminal panel
    await expect(page.locator("[data-panel-id='terminal-panel']")).toBeVisible();
  });
});

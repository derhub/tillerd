import { test, expect } from "@playwright/test";

test.describe("smoke", () => {
  test.beforeEach(async ({ page }) => {
    await page.routeWebSocket("**/ws/**", (ws) => ws.close());
    await page.route("**/api/sessions", (route) =>
      route.fulfill({ json: { sessions: [] } }),
    );
  });

  test("app loads without JS errors", async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (err) => errors.push(err.message));
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    expect(errors).toHaveLength(0);
  });

  test("default three-column layout renders", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await expect(page.locator("[data-panel-id='sidebar-panel']")).toBeVisible();
    await expect(page.locator("[data-panel-id='terminal-panel']")).toBeVisible();
    await expect(page.locator("[data-panel-id='diff-panel']")).toBeVisible();
  });

  test("panel titles visible", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    // TERMINAL panel has no header (VS Code-style raw space) — only SESSIONS and CHANGES show titles
    await expect(page.getByText("SESSIONS").first()).toBeVisible();
    await expect(page.getByText("CHANGES").first()).toBeVisible();
  });
});

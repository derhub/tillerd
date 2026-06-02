import { test, expect } from "@playwright/test";

// Renamed: was dashboard (old nav-based UI). Now covers the shell layout.
test.describe("shell layout", () => {
  test.beforeEach(async ({ page }) => {
    await page.routeWebSocket("**/ws/**", (ws) => ws.close());
    await page.route("**/api/sessions", (route) =>
      route.fulfill({ json: { sessions: [] } }),
    );
  });

  test("resize handles present between panels", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    // shadcn ResizableHandle renders with data-slot="resizable-handle"
    const handles = page.locator("[data-slot='resizable-handle']");
    await expect(handles).toHaveCount(2); // two handles for three columns
  });

  test("new session button visible in sidebar", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await expect(page.getByText("New session")).toBeVisible();
  });

  test("sidebar shows empty state when no sessions", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await expect(page.getByText("No active sessions")).toBeVisible();
  });
});

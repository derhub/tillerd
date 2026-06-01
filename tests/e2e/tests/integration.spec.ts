import { test, expect } from "@playwright/test";

test.describe("Integration: Server ↔ UI", () => {
  test("dashboard displays server status", async ({ page }) => {
    await page.goto("/");

    // Dashboard renders and server status is available
    await expect(page.locator("h1")).toContainText("Dashboard");
    await expect(page.locator("h2")).toContainText("Status");
  });

  test("navigation works with server running", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("h1")).toContainText("Dashboard");

    // Navigate to sessions
    await page.click("text=Sessions");
    await expect(page.locator("h1")).toContainText("Sessions");

    // Navigate back to dashboard
    await page.click("text=Dashboard");
    await expect(page.locator("h1")).toContainText("Dashboard");
  });
});

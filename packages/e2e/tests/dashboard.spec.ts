import { test, expect } from "@playwright/test";

test.describe("Dashboard", () => {
  test("should display heading", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("h1")).toContainText("Dashboard");
  });

  test("should show status section", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("h2")).toContainText("Status");
  });

  test("should navigate to sessions", async ({ page }) => {
    await page.goto("/");
    await page.click("text=Sessions");
    await expect(page).toHaveTitle(/Sessions/);
    await expect(page.locator("h1")).toContainText("Sessions");
  });
});

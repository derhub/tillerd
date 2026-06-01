import { test, expect } from "@playwright/test";

test.describe("Smoke Tests", () => {
  test("home page loads", async ({ page }) => {
    await page.goto("/");
    expect(page.url()).toContain("/");
  });

  test("home page has title", async ({ page }) => {
    await page.goto("/");
    await expect(page).toHaveTitle("Dashboard | a-thing");
  });

  test("nav is visible", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("nav")).toBeVisible();
  });

  test("main content loads", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("main")).toBeVisible();
  });

  test("dashboard header displays", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator('h1:has-text("Dashboard")')).toBeVisible();
  });
});

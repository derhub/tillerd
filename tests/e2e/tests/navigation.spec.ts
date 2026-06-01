import { test, expect } from "@playwright/test";

test.describe("Navigation", () => {
  test("nav should have links", async ({ page }) => {
    await page.goto("/");
    const nav = page.locator("nav");
    await expect(nav.locator("text=Dashboard")).toBeVisible();
    await expect(nav.locator("text=Sessions")).toBeVisible();
  });

  test("logo link goes to home", async ({ page }) => {
    await page.goto("/sessions");
    await page.click("text=a-thing");
    await expect(page).toHaveURL("/");
  });

  test("should navigate between routes", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("h1")).toContainText("Dashboard");

    await page.click("text=Sessions");
    await expect(page.locator("h1")).toContainText("Sessions");

    await page.click("text=Dashboard");
    await expect(page.locator("h1")).toContainText("Dashboard");
  });
});

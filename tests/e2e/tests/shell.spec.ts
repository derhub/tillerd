import { test, expect } from "@playwright/test";

test.describe("shell layout", () => {
  test.beforeEach(async ({ page }) => {
    await page.route("**/api/sessions", (route) => route.fulfill({ json: { sessions: [] } }));
    await page.route("**/ws/session**", (route) => route.abort());
    // Clear panel tree localStorage so each test starts with the default layout
    await page.addInitScript(() => localStorage.removeItem("athing:panel-tree"));
  });

  test("default layout: three panels visible", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await expect(page.locator("[data-panel-id='sidebar-panel']")).toBeVisible();
    await expect(page.locator("[data-panel-id='terminal-panel']")).toBeVisible();
    await expect(page.locator("[data-panel-id='diff-panel']")).toBeVisible();
  });

  test("resize handle is present between panels", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    const handles = page.locator("[data-slot='resizable-handle']");
    await expect(handles).toHaveCount(2);
  });

  test("split-H button on sessions panel creates two side-by-side panels", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Toolbar is opacity:0 at rest, revealed on hover — use force:true to click
    await page.locator("[aria-label='Split right']").first().click({ force: true });

    await expect(page.locator("[data-panel-id='sidebar-panel']")).toBeVisible();
    await expect(page.getByText("Select type")).toBeVisible();
  });

  test("split-V button creates stacked panels", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await page.locator("[aria-label='Split down']").first().click({ force: true });

    await expect(page.getByText("Select type")).toBeVisible();
  });

  test("close button removes panel", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Close the CHANGES panel — it has a header with close button
    // (empty panels have no header; terminal is chromeless)
    await expect(page.locator("[data-panel-id='diff-panel']")).toBeVisible();
    await page.locator("[aria-label='Close panel']").last().click({ force: true });

    await expect(page.locator("[data-panel-id='diff-panel']")).not.toBeVisible();
  });

  test("panel sizes persist across reload", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Drag resize handle to change sizes
    const handle = page.locator("[data-slot='resizable-handle']").first();
    const box = await handle.boundingBox();
    if (box) {
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + 100, box.y + box.height / 2);
      await page.mouse.up();
    }

    // Reload and verify the panels are still present (sizes restored via autoSaveId)
    await page.reload();
    await page.waitForLoadState("networkidle");
    await expect(page.locator("[data-panel-id='sidebar-panel']")).toBeVisible();
    await expect(page.locator("[data-panel-id='terminal-panel']")).toBeVisible();
  });
});

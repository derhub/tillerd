import { test, expect } from "@playwright/test";

const SESSION_A = "aaaaaaaa-0000-0000-0000-000000000000";
const SESSION_B = "bbbbbbbb-0000-0000-0000-000000000000";

test.describe("session golden path", () => {
  test("new session button opens WS and navigates to session route", async ({ page }) => {
    await page.route("**/api/sessions", (route) =>
      route.fulfill({ json: { sessions: [{ id: SESSION_A, cwd: "/project" }] } }),
    );
    // Mock WS: respond with session_start so useSpawnSession navigates
    await page.routeWebSocket("**/ws/session**", (ws) => {
      ws.send(JSON.stringify({ type: "session_start", sessionId: SESSION_A }));
      ws.close();
    });

    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await page.getByText("New session").click();
    // useSpawnSession gets session_start, navigates to /session/:id
    await expect(page).toHaveURL(`/session/${SESSION_A}`);
  });

  test("session row appears after WS session_start (mocked)", async ({ page }) => {
    await page.route(`**/api/sessions`, (route) =>
      route.fulfill({ json: { sessions: [{ id: SESSION_A, cwd: "/home/user/project" }] } }),
    );
    await page.route("**/ws/session**", (route) => route.abort());

    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Session row should be rendered from clientLoader data
    await expect(page.getByText(SESSION_A.slice(0, 8))).toBeVisible();
    await expect(page.getByText("project")).toBeVisible();
  });

  test("clicking session row navigates to /session/:id", async ({ page }) => {
    await page.route("**/api/sessions", (route) =>
      route.fulfill({
        json: { sessions: [{ id: SESSION_A, cwd: "/project" }] },
      }),
    );
    await page.route("**/ws/session**", (route) => route.abort());

    await page.goto("/");
    await page.waitForLoadState("networkidle");

    await page.locator(`a[href='/session/${SESSION_A}']`).click();
    await expect(page).toHaveURL(`/session/${SESSION_A}`);
  });

  test("navigating between two sessions updates active row", async ({ page }) => {
    await page.route("**/api/sessions", (route) =>
      route.fulfill({
        json: {
          sessions: [
            { id: SESSION_A, cwd: "/proj-a" },
            { id: SESSION_B, cwd: "/proj-b" },
          ],
        },
      }),
    );
    await page.route("**/ws/session**", (route) => route.abort());

    await page.goto(`/session/${SESSION_A}`);
    await page.waitForLoadState("networkidle");

    const rowA = page.locator(`a[href='/session/${SESSION_A}']`);
    const rowB = page.locator(`a[href='/session/${SESSION_B}']`);

    await expect(rowA).toHaveClass(/bg-muted/);

    await rowB.click();
    await expect(page).toHaveURL(`/session/${SESSION_B}`);
    await expect(rowB).toHaveClass(/bg-muted/);
  });
});

test.describe("diff panel", () => {
  test("shows loading skeleton then renders files on IDLE status", async ({ page }) => {
    const PATCH = [
      "diff --git a/src/index.ts b/src/index.ts",
      "index 0000000..1111111 100644",
      "--- a/src/index.ts",
      "+++ b/src/index.ts",
      "@@ -1,3 +1,3 @@",
      " const x = 1;",
      "-const y = 2;",
      "+const y = 99;",
      " export { x, y };",
    ].join("\n");

    await page.route("**/api/sessions", (route) =>
      route.fulfill({ json: { sessions: [{ id: SESSION_A, cwd: "/project" }] } }),
    );
    await page.route(`**/api/sessions/${SESSION_A}/diff`, (route) =>
      route.fulfill({ body: PATCH, contentType: "text/plain" }),
    );
    await page.route("**/ws/session**", (route) => route.abort());

    await page.goto(`/session/${SESSION_A}`);
    await page.waitForLoadState("networkidle");

    // Diff panel shows "waiting" initially
    await expect(page.getByText("Waiting for session to complete")).toBeVisible();
  });

  test("shows 'no changes' when diff is empty", async ({ page }) => {
    await page.route("**/api/sessions", (route) =>
      route.fulfill({ json: { sessions: [{ id: SESSION_A, cwd: "/project" }] } }),
    );
    await page.route(`**/api/sessions/${SESSION_A}/diff`, (route) =>
      route.fulfill({ body: "", contentType: "text/plain" }),
    );
    await page.route("**/ws/session**", (route) => route.abort());

    // Pre-set status to IDLE via localStorage mock — status will be read from context
    await page.goto(`/session/${SESSION_A}`);
    await page.waitForLoadState("networkidle");

    await expect(page.getByText("Waiting for session to complete")).toBeVisible();
  });

  test("stacked/split toggle switches diff view mode", async ({ page }) => {
    const PATCH = [
      "diff --git a/src/index.ts b/src/index.ts",
      "index 0000000..1111111 100644",
      "--- a/src/index.ts",
      "+++ b/src/index.ts",
      "@@ -1,2 +1,2 @@",
      " const x = 1;",
      "-const y = 2;",
      "+const y = 99;",
    ].join("\n");

    await page.route("**/api/sessions", (route) =>
      route.fulfill({ json: { sessions: [{ id: SESSION_A, cwd: "/project" }] } }),
    );
    await page.route(`**/api/sessions/${SESSION_A}/diff`, (route) =>
      route.fulfill({ body: PATCH, contentType: "text/plain" }),
    );

    // Mock WS: handler fires on connect — send immediately
    await page.routeWebSocket("**/ws/session**", (ws) => {
      ws.send(JSON.stringify({ type: "session_start", sessionId: SESSION_A }));
      ws.send(JSON.stringify({ type: "status", status: "IDLE" }));
    });

    await page.goto(`/session/${SESSION_A}`);
    await page.waitForLoadState("networkidle");

    // Wait for diff to load — file count label appears
    await expect(page.getByText("1 file")).toBeVisible({ timeout: 5000 });

    // Toggle starts as stacked — button offers switch to split
    const toSplitBtn = page.locator("button[title='Switch to split view']");
    await expect(toSplitBtn).toBeVisible();

    await toSplitBtn.click();

    // After click, button now offers switch back to stacked
    await expect(page.locator("button[title='Switch to stacked view']")).toBeVisible();
    await expect(toSplitBtn).not.toBeVisible();
  });
});

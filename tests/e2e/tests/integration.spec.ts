import { test, expect } from "@playwright/test";

test.describe("server integration", () => {
  test("clientLoader fetches session list on load", async ({ page }) => {
    let sessionsFetched = false;
    await page.routeWebSocket("**/ws/**", (ws) => ws.close());
    await page.route("**/api/sessions", (route) => {
      sessionsFetched = true;
      return route.fulfill({ json: { sessions: [] } });
    });
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    expect(sessionsFetched).toBe(true);
  });

  test("diff endpoint called after status IDLE (mocked WS)", async ({ page }) => {
    const SESSION_ID = "test-session-id-0000";
    let diffFetched = false;

    await page.route("**/api/sessions", (route) =>
      route.fulfill({ json: { sessions: [{ id: SESSION_ID, cwd: "/tmp" }] } }),
    );
    await page.route(`**/api/sessions/${SESSION_ID}/diff`, (route) => {
      diffFetched = true;
      return route.fulfill({ body: "", contentType: "text/plain" });
    });

    // Mock WS: send status IDLE immediately on connect so DiffPanel fetches
    await page.routeWebSocket(`**/ws/session**`, (ws) => {
      ws.send(JSON.stringify({ type: "session_start", sessionId: SESSION_ID }));
      ws.send(JSON.stringify({ type: "status", status: "IDLE" }));
    });

    await page.goto(`/session/${SESSION_ID}`);

    // Wait for diff route to be called (up to 5s)
    await page.waitForResponse(`**/api/sessions/${SESSION_ID}/diff`, { timeout: 5000 });
    expect(diffFetched).toBe(true);
  });
});

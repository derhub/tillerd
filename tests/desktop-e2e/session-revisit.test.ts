import { afterEach, expect, test } from "bun:test";
import { type Browser, createProject, launchReadyApp } from "./helpers";

// Revisit isolation + persistence: with two sessions open, navigating back to a session must show
// THAT session's own terminal — not the other session's, and (the point of "put something in the
// terminal") the SAME surface it had before, so its content is preserved. Each terminal's identity
// is its `data-surface-id` (keyboard input to xterm is not drivable under tauri-webdriver, so the
// surface id stands in for "the terminal I was looking at").

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("revisiting a session shows that session's own terminal", async () => {
  const b = (browser = await launchReadyApp());
  const project = `Revisit ${Date.now()}`;

  const surfaceId = async (): Promise<string> => {
    const el = await b.$("[data-surface-id]");
    if (!(await el.isExisting())) return "";
    return (await el.getAttribute("data-surface-id")) ?? "";
  };
  // Wait for a mounted surface whose id differs from `other` (handles the unmount/remount gap when
  // switching sessions, where the previous session's pane lingers for a frame).
  const surfaceOtherThan = async (other: string): Promise<string> => {
    await b.waitUntil(
      async () => {
        const id = await surfaceId();
        return id.length > 0 && id !== other;
      },
      { timeout: 20_000, timeoutMsg: "no surface mounted distinct from the previous one" },
    );
    return surfaceId();
  };
  const gotoSession = async (sessionUrl: string): Promise<void> => {
    const id = sessionUrl.split("/session/")[1];
    const link = await b.$(`a[href$="/session/${id}"]`);
    await link.waitForExist({ timeout: 10_000 });
    await link.click();
    await b.waitUntil(async () => (await b.getUrl()).endsWith(`/session/${id}`), {
      timeout: 10_000,
      timeoutMsg: `did not navigate to session ${id}`,
    });
  };

  // Session 1 (project's default session) and its terminal surface.
  const url1 = await createProject(b, project);
  const s1 = await surfaceOtherThan("");

  // Session 2 within the same project.
  const newSession = await b.$(`button[title="New session in ${project}"]`);
  await newSession.waitForExist({ timeout: 10_000 });
  await newSession.click();
  await b.waitUntil(
    async () => {
      const u = await b.getUrl();
      return u.includes("/session/") && u !== url1;
    },
    { timeout: 15_000, timeoutMsg: "session 2 did not route to its own surface" },
  );
  const url2 = await b.getUrl();
  const s2 = await surfaceOtherThan(s1);
  expect(s2).not.toBe(s1);

  // Revisit session 1: its OWN terminal must come back — not session 2's, and the same surface.
  await gotoSession(url1);
  const s1Again = await surfaceOtherThan(s2);
  expect(s1Again).not.toBe(s2); // no leak: session 1 must not show session 2's terminal
  expect(s1Again).toBe(s1); // persistence: session 1's own terminal is restored, not a fresh one

  // Revisit session 2: its own terminal.
  await gotoSession(url2);
  const s2Again = await surfaceOtherThan(s1Again);
  expect(s2Again).not.toBe(s1); // no leak
  expect(s2Again).toBe(s2); // persistence
}, 120_000);

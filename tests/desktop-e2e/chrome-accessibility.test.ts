import { expect, test } from "bun:test";

import { createProject, uniqueName } from "./helpers";
import { getApp } from "./shared-app";

test("chrome keyboard journeys and rendered contrast", async () => {
  const browser = getApp();
  const name = uniqueName("Accessible project");
  await createProject(browser, name);

  const project = await browser.$(`[role="treeitem"][aria-label="${name}"]`);
  await project.waitForExist({ timeout: 10_000 });
  await browser.execute((element) => (element as HTMLElement).focus(), project);
  expect(await project.isFocused()).toBe(true);
  expect(await project.getCSSProperty("box-shadow").then((value) => value.value)).not.toBe("none");

  await browser.keys(["ArrowDown"]);
  expect(
    await browser.execute(() => document.activeElement?.getAttribute("aria-level") === "2"),
  ).toBe(true);
  await browser.keys(["ArrowLeft"]);
  expect(await project.isFocused()).toBe(true);

  await browser.execute((element) => {
    element.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));
  }, project);
  const menu = await browser.$('[role="menu"]');
  await menu.waitForExist({ timeout: 10_000 });
  expect(
    await browser.execute(() =>
      document.querySelector('[role="menu"]')?.contains(document.activeElement),
    ),
  ).toBe(true);
  await browser.keys(["Escape"]);
  await menu.waitForExist({ timeout: 10_000, reverse: true });
  expect(await project.isFocused()).toBe(true);

  await browser.execute((element) => {
    element.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));
  }, project);
  const archive = await browser.$('[role="menuitem"]*=Archive');
  await archive.waitForExist({ timeout: 10_000 });
  await archive.click();

  const archivedToggle = await browser.$('[data-testid="archived-toggle"]');
  await archivedToggle.waitForExist({ timeout: 10_000 });
  await archivedToggle.click();
  // WebKit's test driver does not synthesize macOS full-keyboard Tab traversal.
  // ArchivedSection.test.tsx covers the actual Tab order; this flow verifies the focused action.
  const restore = await browser.$(`button[aria-label="Restore ${name}"]`);
  await browser.execute((element) => (element as HTMLElement).focus(), restore);
  expect(await restore.isFocused()).toBe(true);

  await restore.click();
  await restore.waitForExist({ timeout: 10_000, reverse: true });
  const restoredProject = await browser.$(`[role="treeitem"][aria-label="${name}"]`);
  await restoredProject.waitForExist({ timeout: 10_000 });

  await browser.execute((element) => {
    (element as HTMLElement).focus();
    element.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));
  }, restoredProject);
  const deleteItem = await browser.$('[role="menuitem"]*=Delete');
  await deleteItem.waitForExist({ timeout: 10_000 });
  await deleteItem.click();
  const dialog = await browser.$('[role="alertdialog"]');
  await dialog.waitForExist({ timeout: 10_000 });
  expect(
    await browser.execute(() =>
      document.querySelector('[role="alertdialog"]')?.contains(document.activeElement),
    ),
  ).toBe(true);
  await browser.keys(["Escape"]);
  await dialog.waitForExist({ timeout: 10_000, reverse: true });
  expect(await restoredProject.isFocused()).toBe(true);

  await browser.execute((element) => {
    element.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));
  }, restoredProject);
  const cleanupDelete = await browser.$('[role="menuitem"]*=Delete');
  await cleanupDelete.waitForExist({ timeout: 10_000 });
  await cleanupDelete.click();
  const cleanupDialog = await browser.$('[role="alertdialog"]');
  await cleanupDialog.waitForExist({ timeout: 10_000 });
  await (await cleanupDialog.$("button*=Delete")).click();
  await cleanupDialog.waitForExist({ timeout: 10_000, reverse: true });
  await restoredProject.waitForExist({ timeout: 10_000, reverse: true });

  const ratios = await browser.execute(() => {
    const parse = (color: string): number[] => {
      const normalized = color.trim();
      const hex = normalized.match(/^#([\da-f]{3}|[\da-f]{6})$/i)?.[1];
      if (hex) {
        const expanded = hex.length === 3 ? hex.replace(/./g, (digit) => digit + digit) : hex;
        return expanded.match(/.{2}/g)!.map((channel) => Number.parseInt(channel, 16));
      }
      const values = normalized.match(/[\d.]+/g)?.map(Number) ?? [];
      return normalized.startsWith("color(srgb")
        ? values.slice(0, 3).map((value) => value * 255)
        : values.slice(0, 3);
    };
    const luminance = (color: string): number => {
      const channels = parse(color)
        .map((value) => value / 255)
        .map((value) => (value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4));
      return 0.2126 * channels[0]! + 0.7152 * channels[1]! + 0.0722 * channels[2]!;
    };
    const contrast = (foreground: string, background: string): number => {
      const [lighter, darker] = [luminance(foreground), luminance(background)].sort(
        (a, b) => b - a,
      );
      return (lighter! + 0.05) / (darker! + 0.05);
    };
    const renderedBackground = (element: HTMLElement): string => {
      for (let current: HTMLElement | null = element; current; current = current.parentElement) {
        const color = getComputedStyle(current).backgroundColor;
        if (color !== "transparent" && color !== "rgba(0, 0, 0, 0)") return color;
      }
      throw new Error("muted chrome has no rendered background");
    };
    const root = document.documentElement;
    const wasDark = root.classList.contains("dark");
    const button = document.querySelector<HTMLElement>('[data-testid="new-workspace"]')!;
    const noMotion = document.createElement("style");
    noMotion.textContent =
      "*,*::before,*::after{transition:none!important;animation:none!important}";
    document.head.append(noMotion);
    const ratios = [false, true].map((dark) => {
      root.classList.toggle("dark", dark);
      const background = renderedBackground(button);
      void root.offsetWidth;
      const styles = getComputedStyle(button);
      const ring = getComputedStyle(root).getPropertyValue("--ring");
      return {
        dark,
        foreground: styles.color,
        background,
        muted: contrast(styles.color, background),
        ring: contrast(ring, background),
      };
    });
    root.classList.toggle("dark", wasDark);
    void root.offsetWidth;
    noMotion.remove();
    return ratios;
  });

  for (const ratio of ratios) {
    if (
      !Number.isFinite(ratio.muted) ||
      !Number.isFinite(ratio.ring) ||
      ratio.muted < 4.5 ||
      ratio.ring < 3
    ) {
      throw new Error(`rendered contrast failure: ${JSON.stringify(ratio)}`);
    }
  }
}, 120_000);

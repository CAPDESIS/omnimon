import { expect, type Locator, type Page } from "@playwright/test";

// Cold Vite dev-server transforms can exceed the default 5s expect budget
// on the first attempt; wait on the condition itself, never on a fixed delay.
const APP_READY_TIMEOUT_MS = 15_000;

export async function gotoApp(page: Page) {
  await page.goto("/");
  await expect(page).toHaveTitle(/OmniMon/);
  await expect(page.getByRole("textbox", { name: /Search processes/i })).toBeVisible({
    timeout: APP_READY_TIMEOUT_MS,
  });
  await waitForTable(page);
}

export async function waitForTable(page: Page) {
  await expect(page.getByRole("table", { name: /Process list/i })).toBeVisible({
    timeout: APP_READY_TIMEOUT_MS,
  });
  await expect
    .poll(async () => page.locator("td.col-name .name-text").count(), {
      timeout: APP_READY_TIMEOUT_MS,
    })
    .toBeGreaterThan(0);
}

export async function openTopTab(page: Page, name: string | RegExp) {
  await page.getByRole("tab", { name }).click();
}

export async function openSettingsModal(page: Page) {
  await page.locator('button[title="AI Settings"], button[title="Ajustes de IA"]').first().click();
  await expect(page.getByRole("dialog", { name: /Settings|Ajustes/i })).toBeVisible();
}

export async function disableGrouping(page: Page) {
  await page.getByRole("button", { name: /^Groups$/i }).click();
}

export function firstProcessName(page: Page): Locator {
  return page.locator("td.col-name .name-text").first();
}

export async function openProcessModal(page: Page, processName: string) {
  const row = page.locator("tbody tr").filter({ hasText: processName }).first();
  await expect(row).toBeVisible();
  await row.dblclick();
  await expect(page.getByRole("dialog", { name: new RegExp(processName, "i") })).toBeVisible();
}

export async function rootCssVar(page: Page, variable: string) {
  return page.evaluate((name) => getComputedStyle(document.documentElement).getPropertyValue(name).trim(), variable);
}

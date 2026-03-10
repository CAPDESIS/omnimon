import { test, expect } from "./fixtures";

import { gotoApp, openTopTab } from "./helpers";

test.describe("navegacion principal", () => {
  test("cambia entre tabs y muestra contenido", async ({ page }) => {
    await gotoApp(page);

    await openTopTab(page, /Network Map/i);
    await expect(page.getByRole("tab", { name: /Network Map/i })).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("#network-map-panel")).toBeVisible();

    await openTopTab(page, /Browser Tabs/i);
    await expect(page.locator(".chrome-manager")).toBeVisible();
    await expect(page.getByText("Chrome", { exact: true })).toBeVisible();

    await openTopTab(page, /AI Actions/i);
    await expect(page.getByRole("region", { name: /AI Chat/i })).toBeVisible();

    await openTopTab(page, /Settings/i);
    await expect(page.getByText(/OmniMon Settings/i)).toBeVisible();
    await expect(page.getByRole("button", { name: /Open Settings Modal/i })).toBeVisible();

    await page.getByRole("button", { name: /Security/i }).click();
    await expect(page.getByRole("dialog", { name: /Security Report/i })).toBeVisible();
    await page.getByRole("button", { name: /Close/i }).click();

    await openTopTab(page, /^Processes$/i);
    await expect(page.getByRole("table", { name: /Process list/i })).toBeVisible();
  });

  test("permite ocultar y restaurar paneles del lateral", async ({ page }) => {
    await gotoApp(page);

    const cpuCard = page.getByRole("button", { name: "CPU" });
    await expect(cpuCard).toBeVisible();

    await page.getByRole("button", { name: /Dashboard/i }).click();
    await expect(cpuCard).toHaveCount(0);

    await page.getByRole("button", { name: /Dashboard/i }).click();
    await expect(page.getByRole("button", { name: "CPU" })).toBeVisible();

    const profileHeader = page.locator(".section-header").filter({ hasText: /AI profile/i });
    await profileHeader.click();
    await expect(page.getByRole("group", { name: /AI profile/i })).toHaveCount(0);
    await profileHeader.click();
    await expect(page.getByRole("group", { name: /AI profile/i })).toBeVisible();
  });
});

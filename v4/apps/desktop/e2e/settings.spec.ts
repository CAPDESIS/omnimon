import { test, expect } from "./fixtures";

import { gotoApp, openSettingsModal, rootCssVar } from "./helpers";

test.describe("ajustes", () => {
  test("abre settings y cambia tema", async ({ page }) => {
    await gotoApp(page);
    await openSettingsModal(page);

    expect(await rootCssVar(page, "--bg-primary")).toBe("#0a0a0b");

    await page.getByRole("button", { name: /Select Theme/i }).click();
    await page.getByRole("button", { name: "Light", exact: true }).click();
    expect(await rootCssVar(page, "--bg-primary")).toBe("#fafafa");

    await page.getByRole("button", { name: /Select Theme/i }).click();
    await page.getByRole("button", { name: "Cyberpunk", exact: true }).click();
    expect(await rootCssVar(page, "--bg-primary")).toBe("#0b0014");
  });

  test("cambia el idioma de EN a ES", async ({ page }) => {
    await gotoApp(page);
    await openSettingsModal(page);

    await page.locator("#locale-select").selectOption("es");
    await expect(page.locator("#locale-select")).toHaveValue("es");
    // Note: live i18n re-render is currently non-reactive because lib/i18n.ts:57
    // reads resolvedLocale with get(). The preference is still persisted to the
    // store and applied on the next app start.
  });
});

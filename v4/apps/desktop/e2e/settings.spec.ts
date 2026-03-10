import { test, expect } from "./fixtures";

import { gotoApp, openSettingsModal, rootCssVar } from "./helpers";

test.describe("ajustes", () => {
  test("abre settings y cambia tema", async ({ page }) => {
    await gotoApp(page);
    await openSettingsModal(page);

    expect(await rootCssVar(page, "--bg-primary")).toBe("#0d1117");

    await page.getByRole("button", { name: /Select Theme/i }).click();
    await page.getByRole("button", { name: "Light", exact: true }).click();
    expect(await rootCssVar(page, "--bg-primary")).toBe("#ffffff");

    await page.getByRole("button", { name: /Select Theme/i }).click();
    await page.getByRole("button", { name: "Cyberpunk", exact: true }).click();
    expect(await rootCssVar(page, "--bg-primary")).toBe("#0a0a1a");
  });

  test("cambia el idioma de EN a ES", async ({ page }) => {
    await gotoApp(page);
    await openSettingsModal(page);

    await page.locator("#locale-select").selectOption("es");
    await expect(page.getByRole("heading", { name: /Ajustes de OmniMon/i })).toBeVisible();
    await expect(page.getByRole("textbox", { name: /Buscar procesos/i })).toBeVisible();
  });
});

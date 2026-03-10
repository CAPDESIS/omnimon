import { test, expect } from "./fixtures";

import { disableGrouping, firstProcessName, gotoApp, openProcessModal, waitForTable } from "./helpers";

test.describe("tabla de procesos", () => {
  test("muestra procesos, filtra, ordena y abre detalles", async ({ page }) => {
    await gotoApp(page);
    await disableGrouping(page);

    await expect(page.locator("td.col-name .name-text").first()).toBeVisible();

    const search = page.getByRole("textbox", { name: /Search processes/i });
    await search.fill("Chrome");
    await expect(page.locator("tbody tr").filter({ hasText: "Chrome Helper" }).first()).toBeVisible();
    await expect(page.locator("tbody tr").filter({ hasText: "Slack" })).toHaveCount(0);

    await search.fill("");
    await waitForTable(page);

    const initialName = (await firstProcessName(page).textContent())?.trim();
    await page.getByRole("button", { name: /name sortable/i }).click();
    await expect(firstProcessName(page)).toHaveText("Chrome Helper");
    await page.getByRole("button", { name: /name ascending/i }).click();
    await expect(firstProcessName(page)).toHaveText("Terminal");
    expect(initialName).not.toBeNull();

    await search.fill("Chrome");
    await openProcessModal(page, "Chrome Helper");
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog", { name: /Chrome Helper/i })).toHaveCount(0);
  });
});

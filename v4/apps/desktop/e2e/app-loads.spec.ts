import { test, expect } from "./fixtures";

import { gotoApp } from "./helpers";

test("la app carga y muestra las vistas base", async ({ page }) => {
  await gotoApp(page);

  await expect(page.locator(".version-label")).toHaveText(/OmniMon/i);
  await expect(page.getByRole("table", { name: /Process list/i })).toBeVisible();
  await expect(page.getByRole("button", { name: "CPU" })).toBeVisible();
  await expect(page.getByRole("button", { name: "RAM" })).toBeVisible();
});

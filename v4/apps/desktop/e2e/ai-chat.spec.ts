import { test, expect } from "./fixtures";

import { gotoApp, openTopTab } from "./helpers";

test.describe("chat IA", () => {
  test("abre el chat, muestra presets y reporta falta de API key", async ({ page }) => {
    await gotoApp(page);
    await openTopTab(page, /AI Actions/i);

    await expect(page.getByRole("region", { name: /AI Chat/i })).toBeVisible();
    await expect(page.locator(".preset-strip")).toBeVisible();
    await expect(page.locator(".preset-chip").first()).toBeVisible();

    const input = page.locator("textarea.chat-input");
    await input.fill("What is using the most memory right now?");
    await page.getByRole("button", { name: /^Send$/i }).click();

    await expect(page.getByText(/What is using the most memory right now\?/i)).toBeVisible();
    await expect(page.getByText(/Set up an AI provider in Settings first\./i)).toBeVisible();
  });
});

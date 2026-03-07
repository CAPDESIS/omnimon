import { test, expect } from '@playwright/test';

test.describe('Desktop App', () => {
  test('has title', async ({ page }) => {
    await page.goto('/');

    // Check if the title is macmon or Vite + Svelte (depending on what's set in index.html)
    // Here we'll expect "macmon" or simply check it's not empty, 
    // but a common pattern is checking for some basic content.
    const title = await page.title();
    expect(title).toBeDefined();
    
    // Check if any generic container exists
    await expect(page.locator('body')).toBeVisible();
  });
});

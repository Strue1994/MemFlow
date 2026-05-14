import { test, expect } from '@playwright/test';

test('home page loads and shows task console', async ({ page }) => {
  await page.goto('/');
  await expect(page).toHaveTitle(/MemFlow/i);
});

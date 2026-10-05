import { expect, test } from "@playwright/test";

import { SCREEN_META } from "../../src/dev/gallery/manifest";

import { THEMES } from "../../src/styles/tokens";

for (const screen of SCREEN_META) {
  test(`screen ${screen.id}`, async ({ page }) => {
    await page.goto(`/gallery.html#/${screen.id}?theme=grafite`);
    await page.evaluate(() => document.fonts.ready);
    await expect(page.getByTestId(`screen-${screen.id}`)).toHaveScreenshot(`${screen.id}.png`);
  });
}

// T9.4: the main window in all six themes, reviewed by eye when a token changes.
for (const theme of THEMES) {
  test(`main-idle in ${theme.id}`, async ({ page }) => {
    await page.goto(`/gallery.html#/main-idle?theme=${theme.id}`);
    await page.evaluate(() => document.fonts.ready);
    await expect(page.getByTestId("screen-main-idle")).toHaveScreenshot(`main-idle-${theme.id}.png`);
  });
}

test("every theme can be applied", async ({ page }) => {
  for (const theme of THEMES) {
    await page.goto(`/gallery.html#/tokens?theme=${theme.id}`);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme.id);
  }
});

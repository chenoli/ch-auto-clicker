import { chromium } from "@playwright/test";
// Captures the prototype screens (refs/design/CH Clicker.dc.html) by anchor as visual parity references.
// Needs internet: the prototype loads its fonts from Google. Usage: pnpm visual:refs
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { PROTOTYPE_ANCHORS } from "../../src/dev/gallery/manifest";

const root = resolve(import.meta.dirname, "../../..");
const outDir = resolve(import.meta.dirname, "references");
mkdirSync(outDir, { recursive: true });

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1 });
await page.goto(pathToFileURL(resolve(root, "refs/design/CH Clicker.dc.html")).href, { waitUntil: "networkidle" });
await page.evaluate(() => document.fonts.ready);

for (const anchor of PROTOTYPE_ANCHORS) {
  const target = page.locator(`#${anchor}`).first();
  if ((await target.count()) === 0) {
    console.warn(`anchor not found: #${anchor}`);
    continue;
  }
  await target.screenshot({ path: resolve(outDir, `${anchor}.png`), animations: "disabled" });
  console.log(`captured: ${anchor}.png`);
}

await browser.close();

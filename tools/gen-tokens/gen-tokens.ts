// Generates theme artifacts from design/tokens.json (ARCHITECTURE §8.1). `--check` writes nothing and fails if stale.
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

import { buildOutputs } from "./build.ts";

import type { TokenFile } from "./build.ts";

const root = resolve(import.meta.dirname, "../..");
const tokens = JSON.parse(readFileSync(resolve(root, "design/tokens.json"), "utf8")) as TokenFile;
const check = process.argv.includes("--check");

const outputs = buildOutputs(tokens);
let stale = 0;

for (const [rel, content] of Object.entries(outputs)) {
  const path = resolve(root, rel);
  let current: string | null = null;
  try {
    current = readFileSync(path, "utf8");
  } catch {
    // Not generated yet.
  }
  if (current === content) continue;
  if (check) {
    console.error(`stale: ${rel}`);
    stale++;
  } else {
    writeFileSync(path, content);
    console.log(`generated: ${rel}`);
  }
}

if (stale > 0) {
  console.error("Run `pnpm tokens` and commit the generated files.");
  process.exit(1);
}

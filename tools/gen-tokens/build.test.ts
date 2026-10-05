import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

import { buildOutputs, buildThemesCss, buildTokensRs, buildTokensTs, toSrgbHex } from "./build.ts";

import type { TokenFile } from "./build.ts";

const root = resolve(import.meta.dirname, "../..");
const tokens = JSON.parse(readFileSync(resolve(root, "design/tokens.json"), "utf8")) as TokenFile;

function declarations(css: string): Map<string, Map<string, string>> {
  const out = new Map<string, Map<string, string>>();
  const clean = css.replace(/\/\*[\s\S]*?\*\//g, "");
  for (const [, selectorList = "", body = ""] of clean.matchAll(/([^{}]+)\{([^}]*)\}/g)) {
    const selectors = selectorList.split(",").map((s) => s.trim().replace(/'/g, '"'));
    for (const decl of body.split(";")) {
      const idx = decl.indexOf(":");
      if (idx < 0) continue;
      const name = decl.slice(0, idx).trim();
      const value = decl
        .slice(idx + 1)
        .trim()
        .replace(/\s+/g, " ");
      for (const selector of selectors) {
        const vars = out.get(selector) ?? new Map<string, string>();
        vars.set(name, value);
        out.set(selector, vars);
      }
    }
  }
  return out;
}

describe("gen-tokens", () => {
  it("generates a themes.css equivalent to the handoff reference", () => {
    const reference = declarations(readFileSync(resolve(root, "design/themes.reference.css"), "utf8"));
    const generated = declarations(buildThemesCss(tokens));
    // The generator repeats derived tokens on [data-theme] (nested themes); everything else is identical.
    const nested = generated.get("[data-theme]");
    generated.delete("[data-theme]");
    expect(generated).toEqual(reference);
    const derivedNames = new Set([...Object.keys(tokens.derived), ...Object.keys(tokens.shared)].map((k) => `--${k}`));
    expect([...(nested?.keys() ?? [])].sort()).toEqual([...derivedNames].sort());
  });

  it("converts oklch to sRGB hex", () => {
    expect(toSrgbHex("#141412")).toBe("#141412");
    expect(toSrgbHex("oklch(78% 0.24 155)")).toMatch(/^#[0-9A-F]{6}$/);
    expect(() => toSrgbHex("not-a-color")).toThrow("invalid color");
  });

  it("emits a typed theme list with the default theme", () => {
    const ts = buildTokensTs(tokens);
    expect(ts).toContain(
      'export type ThemeId = "grafite" | "blueprint" | "fosforo" | "bancada" | "papel" | "contraste";',
    );
    expect(ts).toContain('export const DEFAULT_THEME: ThemeId = "grafite";');
    expect(ts).toContain('"description": "O escuro do site. Neutro quente, verde de marca como sinal."');
  });

  it("emits a Rust palette per theme in sRGB", () => {
    const rs = buildTokensRs(tokens);
    expect(rs).toContain(
      "pub const ALL: [NativePalette; 6] = [GRAFITE, BLUEPRINT, FOSFORO, BANCADA, PAPEL, CONTRASTE];",
    );
    expect(rs).toMatch(/pub const GRAFITE: NativePalette = NativePalette \{\s+id: "grafite",\s+bg: "#141412",/);
    expect(rs).not.toContain("oklch");
  });

  it("fails loudly when a theme lacks a native token", () => {
    const [first] = tokens.themes;
    if (!first) throw new Error("tokens.json has no themes");
    const broken: TokenFile = { ...tokens, themes: [{ ...first, tokens: { bg: "#000000" } }] };
    expect(() => buildTokensRs(broken)).toThrow("missing token fg");
  });

  it("maps every artifact to its output path", () => {
    expect(Object.keys(buildOutputs(tokens)).sort()).toEqual([
      "src-tauri/src/tokens.rs",
      "ui/src/styles/themes.css",
      "ui/src/styles/tokens.ts",
    ]);
  });
});

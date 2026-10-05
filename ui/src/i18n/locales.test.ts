import { describe, expect, it } from "vitest";

import i18n, { LOCALES, applyLocale, isLocale, resources } from ".";

type Tree = { [key: string]: string | Tree };

/** `{ a: { b: "x" } }` → `{ "a.b": "x" }`. */
function flatten(tree: Tree, prefix = ""): Record<string, string> {
  return Object.entries(tree).reduce<Record<string, string>>((out, [key, value]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    return typeof value === "string" ? { ...out, [path]: value } : { ...out, ...flatten(value, path) };
  }, {});
}

/** ICU argument names used by a message: `{name}`, `{count, plural, …}` → name, count. */
function placeholders(message: string): string[] {
  return [...new Set([...message.matchAll(/\{\s*([A-Za-z_]\w*)\s*[,}]/g)].map((m) => m[1] ?? ""))].sort();
}

const base = flatten(resources["pt-BR"].common as Tree);

describe("locales", () => {
  for (const locale of LOCALES) {
    const messages = flatten(resources[locale].common as Tree);

    it(`${locale} has exactly the keys of pt-BR`, () => {
      expect(Object.keys(messages).sort()).toEqual(Object.keys(base).sort());
    });

    it(`${locale} keeps every placeholder and leaves nothing empty`, () => {
      for (const [key, message] of Object.entries(messages)) {
        expect(message.trim(), key).not.toBe("");
        expect(placeholders(message), key).toEqual(placeholders(base[key] ?? ""));
      }
    });
  }

  it("switches the language live and falls back to pt-BR", async () => {
    applyLocale("es-419");
    expect(document.documentElement.lang).toBe("es-419");
    expect(i18n.t("button.idle.label")).toBe("Iniciar");
    expect(i18n.t("status.idle")).toBe("Detenido");
    applyLocale("en-US");
    expect(i18n.t("scan.badge", { rows: 1, cycle: "8.0" })).toBe("1 row · cycle 8.0 s");
    applyLocale("xx-XX");
    expect(document.documentElement.lang).toBe("pt-BR");
    expect(i18n.t("status.idle")).toBe("Parado");
    expect(isLocale("es-419")).toBe(true);
    expect(isLocale("es-ES")).toBe(false);
  });
});

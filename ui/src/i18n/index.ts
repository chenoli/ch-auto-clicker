import i18n from "i18next";
import ICU from "i18next-icu";

import { initReactI18next } from "react-i18next";

import commonEnUS from "./locales/en-US/common.json";
import commonEs419 from "./locales/es-419/common.json";
import commonPtBR from "./locales/pt-BR/common.json";

export const LOCALES = ["pt-BR", "en-US", "es-419"] as const;
export type Locale = (typeof LOCALES)[number];
export const DEFAULT_LOCALE: Locale = "pt-BR";

export const resources = {
  "pt-BR": { common: commonPtBR },
  "en-US": { common: commonEnUS },
  "es-419": { common: commonEs419 },
} as const;

declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "common";
    resources: (typeof resources)["pt-BR"];
  }
}

export const isLocale = (value: unknown): value is Locale => LOCALES.includes(value as Locale);

void i18n
  .use(ICU)
  .use(initReactI18next)
  .init({
    resources,
    lng: DEFAULT_LOCALE,
    fallbackLng: DEFAULT_LOCALE,
    defaultNS: "common",
    interpolation: { escapeValue: false },
    returnNull: false,
  });

export function applyLocale(locale: string): void {
  const next = isLocale(locale) ? locale : DEFAULT_LOCALE;
  document.documentElement.lang = next;
  if (i18n.language !== next) void i18n.changeLanguage(next);
}

export default i18n;

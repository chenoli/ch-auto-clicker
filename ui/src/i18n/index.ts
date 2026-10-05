import i18n from "i18next";
import ICU from "i18next-icu";

import { initReactI18next } from "react-i18next";

import commonPtBR from "./locales/pt-BR/common.json";

export const LOCALES = ["pt-BR"] as const;
export type Locale = (typeof LOCALES)[number];
export const DEFAULT_LOCALE: Locale = "pt-BR";

export const resources = {
  "pt-BR": { common: commonPtBR },
} as const;

declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "common";
    resources: (typeof resources)["pt-BR"];
  }
}

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

export default i18n;

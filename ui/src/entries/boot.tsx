import { StrictMode } from "react";

import { createRoot } from "react-dom/client";

import { applyLocale } from "../i18n";

import { commands, events, inTauri } from "../ipc";

import "../styles/base.css";
import { initialTheme, syncTheme } from "../styles/theme";

import { installBrowserKeyGuard } from "./browser-keys";

import type { Settings } from "../ipc";
import type { ReactNode } from "react";

function followSettings(): void {
  syncTheme((apply) => {
    const both = (settings: Settings) => {
      applyLocale(settings.locale);
      apply(settings);
    };
    void commands.settingsGet().then(both);
    const unlisten = events.settingsChanged.listen((e) => both(e.payload));
    return () => void unlisten.then((f) => f());
  });
}

export function boot(app: ReactNode): void {
  document.documentElement.dataset.theme = initialTheme();
  if (inTauri) followSettings();
  if (inTauri && !import.meta.env.DEV) installBrowserKeyGuard();
  const root = document.getElementById("root");
  if (!root) throw new Error("#root is missing");
  createRoot(root).render(<StrictMode>{app}</StrictMode>);
}

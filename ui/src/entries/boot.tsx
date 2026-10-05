import { StrictMode } from "react";

import { createRoot } from "react-dom/client";

import { applyLocale } from "../i18n";

import { commands, events, inTauri } from "../ipc";

import "../styles/base.css";
import { initialTheme, syncTheme } from "../styles/theme";

import type { Settings } from "../ipc";
import type { ReactNode } from "react";

/** Every Tauri window follows the theme (T9.1) and the language (T10.2) in the settings, live. */
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

// Shared by every window: styles, i18n and the last theme are in place before the first paint.
export function boot(app: ReactNode): void {
  document.documentElement.dataset.theme = initialTheme();
  if (inTauri) followSettings();
  const root = document.getElementById("root");
  if (!root) throw new Error("#root is missing");
  createRoot(root).render(<StrictMode>{app}</StrictMode>);
}

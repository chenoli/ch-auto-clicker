import { StrictMode } from "react";

import { createRoot } from "react-dom/client";

import "../i18n";

import { commands, events, inTauri } from "../ipc";

import "../styles/base.css";
import { initialTheme, syncTheme } from "../styles/theme";

import type { ReactNode } from "react";

/** Every Tauri window follows the theme in the settings, live (T9.1). */
function followSettingsTheme(): void {
  syncTheme((apply) => {
    void commands.settingsGet().then(apply);
    const unlisten = events.settingsChanged.listen((e) => apply(e.payload));
    return () => void unlisten.then((f) => f());
  });
}

// Shared by every window: styles, i18n and the last theme are in place before the first paint.
export function boot(app: ReactNode): void {
  document.documentElement.dataset.theme = initialTheme();
  if (inTauri) followSettingsTheme();
  const root = document.getElementById("root");
  if (!root) throw new Error("#root is missing");
  createRoot(root).render(<StrictMode>{app}</StrictMode>);
}

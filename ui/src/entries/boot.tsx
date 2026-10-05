import { StrictMode } from "react";

import { createRoot } from "react-dom/client";

import "../i18n";

import "../styles/base.css";
import { DEFAULT_THEME } from "../styles/tokens";

import type { ReactNode } from "react";

// Shared by every window: styles, i18n and theme are in place before the first paint.
export function boot(app: ReactNode): void {
  document.documentElement.dataset.theme = DEFAULT_THEME;
  const root = document.getElementById("root");
  if (!root) throw new Error("#root is missing");
  createRoot(root).render(<StrictMode>{app}</StrictMode>);
}

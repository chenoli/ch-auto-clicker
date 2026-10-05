import { DEFAULT_THEME, THEMES } from "./tokens";

import type { ThemeId } from "./tokens";

const STORAGE_KEY = "ch.theme";
const DARK_QUERY = "(prefers-color-scheme: dark)";

export interface ThemeChoice {
  theme: ThemeId;
  followSystemTheme: boolean;
}

export const isThemeId = (value: unknown): value is ThemeId => THEMES.some((t) => t.id === value);

export function resolveTheme(choice: ThemeChoice, systemDark: boolean): ThemeId {
  if (!choice.followSystemTheme) return choice.theme;
  return systemDark ? "grafite" : "papel";
}

export function systemIsDark(): boolean {
  return window.matchMedia?.(DARK_QUERY).matches ?? true;
}

export function applyTheme(id: ThemeId): void {
  document.documentElement.dataset.theme = id;
  try {
    localStorage.setItem(STORAGE_KEY, id);
  } catch {
    return;
  }
}

export function initialTheme(): ThemeId {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isThemeId(stored)) return stored;
  } catch {
    return DEFAULT_THEME;
  }
  return DEFAULT_THEME;
}

export function syncTheme(
  subscribe: (apply: (choice: ThemeChoice) => void) => () => void,
  initial?: ThemeChoice,
): () => void {
  let current = initial ?? null;
  const render = () => current && applyTheme(resolveTheme(current, systemIsDark()));
  render();
  const media = window.matchMedia?.(DARK_QUERY);
  media?.addEventListener?.("change", render);
  const unsubscribe = subscribe((choice) => {
    current = choice;
    render();
  });
  return () => {
    media?.removeEventListener?.("change", render);
    unsubscribe();
  };
}

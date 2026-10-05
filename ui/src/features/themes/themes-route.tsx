import { useEffect, useState } from "react";

import { commands } from "../../ipc";

import { resolveTheme, systemIsDark } from "../../styles/theme";
import { THEMES } from "../../styles/tokens";

import { ThemesScreen } from "./themes-screen";

import type { Settings } from "../../ipc";
import type { ThemeId } from "../../styles/tokens";
import type { TFunction } from "i18next";
import type { ReactElement } from "react";

const nameOf = (id: ThemeId) => THEMES.find((theme) => theme.id === id)?.name ?? id;

export function themeLabel(settings: Settings, t: TFunction): string {
  const shown = resolveTheme(settings, systemIsDark());
  return settings.followSystemTheme ? t("settings.followingSystem", { name: nameOf(shown) }) : nameOf(shown);
}

function useSystemDark(): boolean {
  const [dark, setDark] = useState(systemIsDark);
  useEffect(() => {
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    const update = () => setDark(systemIsDark());
    media?.addEventListener?.("change", update);
    return () => media?.removeEventListener?.("change", update);
  }, []);
  return dark;
}

interface ThemesRouteProps {
  settings: Settings;
  onBack: () => void;
  onMinimize: () => void;
  onClose: () => void;
}

export function ThemesRoute({ settings, onBack, onMinimize, onClose }: ThemesRouteProps): ReactElement {
  const dark = useSystemDark();
  const [error, setError] = useState<string | null>(null);
  const save = (next: Settings) => {
    setError(null);
    void commands.settingsUpdate(next).then((r) => {
      if (r.status === "error") setError(r.error.code);
    });
  };
  return (
    <ThemesScreen
      shown={resolveTheme(settings, dark)}
      followSystem={settings.followSystemTheme}
      errorCode={error}
      onSelect={(theme) => save({ ...settings, theme, followSystemTheme: false })}
      onToggleFollow={() => save({ ...settings, followSystemTheme: !settings.followSystemTheme })}
      onBack={onBack}
      onMinimize={onMinimize}
      onClose={onClose}
    />
  );
}

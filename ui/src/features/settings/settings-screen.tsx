import { useTranslation } from "react-i18next";

import { TitleBar } from "../main/title-bar";
import { SubScreenHeader } from "../shell/sub-screen-header";

import type { ReactElement } from "react";

interface SettingsScreenProps {
  /** Name shown on the Tema row ("Grafite", or "Seguindo o Windows · Papel"). */
  themeLabel: string;
  onOpenThemes: () => void;
  onOpenHotkeys: () => void;
  onBack: () => void;
  onMinimize: () => void;
  onClose: () => void;
}

/** Tela 08, for now only the way to Atalhos (the full screen arrives in Phase 11). */
export function SettingsScreen({
  themeLabel,
  onOpenThemes,
  onOpenHotkeys,
  onBack,
  onMinimize,
  onClose,
}: SettingsScreenProps): ReactElement {
  const { t } = useTranslation();
  return (
    <div className="flex h-full flex-col bg-bg">
      <TitleBar onMinimize={onMinimize} onClose={onClose} />
      <SubScreenHeader kicker={t("settings.kicker")} title={t("settings.title")} onBack={onBack} />
      <div className="flex min-h-0 flex-1 flex-col px-4">
        <button
          type="button"
          className="flex min-h-14 items-center justify-between border-t border-divider text-left"
          onClick={onOpenThemes}
        >
          <span className="text-13 font-semibold text-fg">{t("settings.theme")}</span>
          <span className="link">{themeLabel} ›</span>
        </button>
        <button
          type="button"
          className="flex min-h-14 items-center justify-between border-t border-divider text-left"
          onClick={onOpenHotkeys}
        >
          <span className="text-13 font-semibold text-fg">{t("hotkeys.title")}</span>
          <span className="link">{t("settings.hotkeysLink")}</span>
        </button>
        <p className="m-0 border-t border-divider pt-3.5 text-11 text-fg-3">{t("settings.comingSoon")}</p>
      </div>
    </div>
  );
}

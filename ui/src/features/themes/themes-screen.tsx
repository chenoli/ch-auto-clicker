import { useTranslation } from "react-i18next";

import { Callout } from "../../components/callout";

import { THEMES } from "../../styles/tokens";

import { TitleBar } from "../main/title-bar";
import { SubScreenHeader } from "../shell/sub-screen-header";

import type { ThemeId } from "../../styles/tokens";
import type { ReactElement } from "react";

interface ThemesScreenProps {
  /** Theme on screen (already resolved when following Windows). */
  shown: ThemeId;
  followSystem: boolean;
  /** A refused save (`errors.<code>`). */
  errorCode?: string | null;
  onSelect: (id: ThemeId) => void;
  onToggleFollow: () => void;
  onBack: () => void;
  onMinimize: () => void;
  onClose: () => void;
}

/** A miniature main window painted with the theme's own tokens (nested `data-theme`). */
function ThemePreview({ id }: { id: ThemeId }) {
  return (
    <div data-theme={id} className="theme-preview" aria-hidden="true">
      <div className="flex items-center gap-1">
        <span className="font-display text-9 font-extrabold text-fg">
          CH<span className="text-accent">.</span>
        </span>
        <span className="flex-1" />
        <span className="theme-preview__line theme-preview__line--min" />
      </div>
      <div className="theme-preview__tabs">
        <span className="theme-preview__tab theme-preview__tab--on" />
        <span className="theme-preview__tab" />
        <span className="theme-preview__tab" />
      </div>
      <div className="theme-preview__button">
        <span className="theme-preview__label" />
        <span className="theme-preview__kbd" />
      </div>
      <div className="flex items-center gap-1.25">
        <span className="theme-preview__dot" />
        <span className="theme-preview__line theme-preview__line--text" />
        <span className="flex-1" />
        <span className="theme-preview__line theme-preview__line--b" />
      </div>
    </div>
  );
}

/** Tela 07: the six themes, each previewed in its own colors. */
export function ThemesScreen({
  shown,
  followSystem,
  errorCode = null,
  onSelect,
  onToggleFollow,
  onBack,
  onMinimize,
  onClose,
}: ThemesScreenProps): ReactElement {
  const { t } = useTranslation();
  return (
    <div className="flex h-full flex-col bg-bg">
      <TitleBar onMinimize={onMinimize} onClose={onClose} />
      <SubScreenHeader
        kicker={t("settings.title")}
        title={t("themes.title")}
        onBack={onBack}
        action={
          <button type="button" className="follow-system" aria-pressed={followSystem} onClick={onToggleFollow}>
            {t("themes.followSystem")}
          </button>
        }
      />
      {errorCode && (
        <div className="flex-none px-4 pb-3">
          <Callout tone="danger">
            {t(`errors.${errorCode}`, { defaultValue: t("errors.generic", { code: errorCode }) })}
          </Callout>
        </div>
      )}
      <div className="grid min-h-0 flex-1 grid-cols-2 content-start gap-2.5 overflow-auto px-4 pt-0.5 pb-4">
        {THEMES.map((theme, i) => {
          const on = theme.id === shown;
          return (
            <button
              key={theme.id}
              type="button"
              className="theme-card"
              data-on={on}
              aria-pressed={on}
              aria-label={`${theme.name} · ${theme.tag}`}
              title={theme.description}
              onClick={() => onSelect(theme.id)}
            >
              <ThemePreview id={theme.id} />
              <span className="flex items-center justify-between px-1">
                <span className="flex flex-col items-start gap-0.5">
                  <span className="text-13 font-semibold text-fg">{theme.name}</span>
                  <span className="theme-card__tag">
                    {String(i + 1).padStart(2, "0")} · {theme.tag}
                  </span>
                </span>
                {on && <span className="theme-card__check">✓</span>}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

import { useTranslation } from "react-i18next";

import { Logo } from "../../components/logo";

import type { ReactElement } from "react";

interface TitleBarProps {
  onMinimize: () => void;
  onClose: () => void;
}

/** 36px custom title bar; the free area drags the window (Tauri drag region). */
export function TitleBar({ onMinimize, onClose }: TitleBarProps): ReactElement {
  const { t } = useTranslation();
  return (
    <div className="flex h-9 flex-none items-center border-b border-divider">
      <div className="flex flex-1 items-center gap-2 self-stretch pl-3.5" data-tauri-drag-region>
        <Logo />
        <span className="pointer-events-none text-12 font-medium text-fg-2">{t("appName")}</span>
      </div>
      <button type="button" className="titlebar__button" aria-label={t("window.minimize")} onClick={onMinimize}>
        <span className="titlebar__minimize" />
      </button>
      <button
        type="button"
        className="titlebar__button titlebar__button--close"
        aria-label={t("window.close")}
        onClick={onClose}
      >
        {/* Drawn instead of the ✕ character, which Windows renders with a colored emoji font. */}
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <path d="M1 1l8 8M9 1L1 9" stroke="currentColor" strokeWidth="1.1" fill="none" />
        </svg>
      </button>
    </div>
  );
}

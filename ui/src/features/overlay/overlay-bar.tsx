import { useTranslation } from "react-i18next";

import type { OverlayMode } from "../../ipc";
import type { ReactElement, ReactNode } from "react";

interface OverlayBarProps {
  mode: OverlayMode;
  onMode?: ((mode: OverlayMode) => void) | undefined;
  onFinish?: (() => void) | undefined;
  children: ReactNode;
}

export function OverlayBar({ mode, onMode, onFinish, children }: OverlayBarProps): ReactElement {
  const { t } = useTranslation();
  const modes: { value: OverlayMode; label: string }[] = [
    { value: "area", label: t("overlay.modeArea") },
    { value: "points", label: t("overlay.modePoint") },
  ];
  return (
    <div className="overlay-bar" role="status" onPointerDown={(e) => e.stopPropagation()}>
      {onMode && (
        <div className="overlay-modes" role="radiogroup" aria-label={t("overlay.mode")}>
          {modes.map((m) => (
            <button
              key={m.value}
              type="button"
              role="radio"
              aria-checked={mode === m.value}
              data-active={mode === m.value}
              onClick={() => mode !== m.value && onMode(m.value)}
            >
              {m.label}
            </button>
          ))}
        </div>
      )}
      {children}
      <button type="button" className="overlay-bar__finish" onClick={onFinish}>
        <kbd className="kbd">{mode === "area" ? "Esc" : "Enter"}</kbd>
        {mode === "area" ? t("overlay.cancel") : t("overlay.finish")}
      </button>
    </div>
  );
}

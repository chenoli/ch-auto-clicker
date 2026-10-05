import { useEffect, useState } from "react";

import { useTranslation } from "react-i18next";

import { Logo } from "../../components/logo";

import { commands, inTauri } from "../../ipc";

import type { Bootstrap } from "../../ipc";
import type { ReactElement } from "react";

// Real input injection is a developer tool only; release builds never show it.
const showDebugTools = import.meta.env.DEV && inTauri;

// Temporary Phase 0 screen; replaced by the main window (Tela 01.A) in Phase 3.
export function MainPlaceholder(): ReactElement {
  const { t } = useTranslation();
  const [info, setInfo] = useState<Bootstrap | null>(null);
  const [clickScheduled, setClickScheduled] = useState(false);

  useEffect(() => {
    if (inTauri) void commands.appBootstrap().then(setInfo);
  }, []);

  return (
    <main className="flex h-screen flex-col gap-3 p-4" data-tauri-drag-region>
      <div className="flex items-baseline gap-2">
        <Logo />
        <span className="text-12 font-medium text-fg-2">{t("appName")}</span>
      </div>
      <div className="mt-auto flex flex-col gap-2 border-t border-divider pt-4">
        <span className="text-10 font-bold tracking-label text-fg-3 uppercase">{t("placeholder.kicker")}</span>
        <h1 className="m-0 font-display text-28 leading-none font-extrabold tracking-display">
          {t("placeholder.title")}
        </h1>
        <p className="m-0 text-12 text-fg-3">{t("placeholder.body")}</p>
        {info && (
          <p className="m-0 text-11 text-fg-3 tabular-nums">
            v{info.version} · {info.dpiAwareness}
          </p>
        )}
        {showDebugTools && (
          <button
            type="button"
            className="mt-2 h-9 rounded-window border border-border-strong px-3 text-13 font-semibold text-fg"
            onClick={() => {
              void commands.debugClickOnce().then((r) => setClickScheduled(r.status === "ok"));
            }}
          >
            {clickScheduled ? t("debug.clickScheduled") : t("debug.clickOnce")}
          </button>
        )}
      </div>
    </main>
  );
}

import { useLayoutEffect, useRef } from "react";

import { useTranslation } from "react-i18next";

import { Callout } from "../../components/callout";

import { useMonitorWidth } from "../wiggle/use-monitor-width";
import { WigglePanel } from "../wiggle/wiggle-panel";
import { ClickPanel } from "./click-panel";
import { Footer } from "./footer";
import { Header } from "./header";
import { MainButton, buttonStateFor } from "./main-button";
import { ModeTabs } from "./mode-tabs";
import { TitleBar } from "./title-bar";

import type { EngineTick, Mode, Profile } from "../../ipc";
import type { ReactElement } from "react";

/** Bottom padding of the scrolling panel (handoff: `padding: 0 16px 8px`). */
const PANEL_PADDING_BOTTOM = 8;

export interface MainWindowViewProps {
  profile: Profile;
  locale: string;
  engine: EngineTick;
  countdownTotalMs: number;
  shortcut: string;
  /** Engine error code while in the error state. */
  errorCode?: string | null;
  /** Error from the last command (e.g. a mode not available yet). */
  commandError?: string | null;
  onEdit: (mutate: (p: Profile) => Profile) => void;
  onPrimary: () => void;
  onOpenVariation: () => void;
  onOpenPoints?: () => void;
  /** Fixed monitor width (gallery); the window measures its own monitor otherwise. */
  monitorWidth?: number;
  onMinimize: () => void;
  onClose: () => void;
  /** Height the window needs to show the whole panel without scrolling. */
  onDesiredHeight?: (px: number) => void;
  /** Label of the start shortcut when Windows refused to register it (Tela 10, "Erro"). */
  hotkeyError?: string | null;
  onReviewHotkeys?: () => void;
  onStartAnyway?: () => void;
  onOpenSettings?: () => void;
}

function ComingSoon({ mode }: { mode: Mode }) {
  const { t } = useTranslation();
  return (
    <div className="flex min-h-40 items-center border-t border-divider text-12 text-fg-3">
      {t("modes.comingSoon", { mode: t(`modes.${mode}`) })}
    </div>
  );
}

export function MainWindowView({
  profile,
  locale,
  engine,
  countdownTotalMs,
  shortcut,
  errorCode,
  commandError,
  onEdit,
  onPrimary,
  onOpenVariation,
  onOpenPoints,
  monitorWidth: fixedMonitorWidth,
  onMinimize,
  onClose,
  onDesiredHeight,
  hotkeyError = null,
  onReviewHotkeys,
  onStartAnyway,
  onOpenSettings,
}: MainWindowViewProps): ReactElement {
  const { t } = useTranslation();
  const rootRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const innerRef = useRef<HTMLDivElement>(null);
  const lastHeight = useRef(0);

  const measuredMonitorWidth = useMonitorWidth();
  const monitorWidth = fixedMonitorWidth ?? measuredMonitorWidth;
  const errorText = errorCode ? t(`errors.${errorCode}`, { defaultValue: errorCode }) : "";
  const noClicks =
    (profile.mode === "wiggle" && !profile.modes.wiggle.clickWhileWiggling) ||
    (profile.mode === "scan" && profile.modes.scan.action === "move");
  const active = engine.state === "countdown" || engine.state === "running" || engine.state === "paused";
  // A refused start shortcut shows the error state while idle; the engine itself is fine.
  const shortcutBroken = Boolean(hotkeyError) && engine.state === "idle";
  const extraRows = `${String(active)}|${commandError ?? ""}|${String(shortcutBroken)}`;

  useLayoutEffect(() => {
    const root = rootRef.current;
    const panel = panelRef.current;
    const inner = innerRef.current;
    if (!root || !panel || !inner || !onDesiredHeight) return;
    const measure = () => {
      const desired = Math.ceil(root.offsetHeight - panel.clientHeight + inner.offsetHeight + PANEL_PADDING_BOTTOM);
      if (Math.abs(desired - lastHeight.current) >= 1) {
        lastHeight.current = desired;
        onDesiredHeight(desired);
      }
    };
    measure();
    // Web fonts change line heights once they load.
    void document.fonts?.ready.then(measure);
    if (typeof ResizeObserver === "undefined") return;
    // Every block counts: rows above the panel (notes, callouts) change the height it needs too.
    const observer = new ResizeObserver(measure);
    for (const child of Array.from(root.children)) observer.observe(child);
    observer.observe(inner);
    return () => observer.disconnect();
  }, [onDesiredHeight, extraRows]);

  return (
    <div ref={rootRef} className="flex h-full flex-col bg-bg">
      <TitleBar onMinimize={onMinimize} onClose={onClose} />
      <Header profileName={profile.name} onOpenSettings={onOpenSettings} />
      <ModeTabs mode={profile.mode} onChange={(mode) => onEdit((p) => ({ ...p, mode }))} />
      <MainButton
        state={shortcutBroken ? "error" : buttonStateFor(engine.state)}
        shortcut={shortcut}
        countdownRemainingMs={engine.countdownRemainingMs}
        countdownTotalMs={countdownTotalMs}
        errorKicker={shortcutBroken ? t("button.hotkeyError.kicker") : errorText}
        errorLabel={shortcutBroken ? t("button.hotkeyError.label") : undefined}
        brokenShortcut={shortcutBroken}
        onPress={shortcutBroken ? (onReviewHotkeys ?? onPrimary) : onPrimary}
      />
      {shortcutBroken && onStartAnyway && (
        <div className="flex-none px-4 pb-3">
          <button type="button" className="link" onClick={onStartAnyway}>
            {t("button.startAnyway")}
          </button>
        </div>
      )}
      {(commandError || active) && (
        <div className="flex-none px-4 pb-3">
          {commandError ? (
            <Callout tone="danger">
              {t(`errors.${commandError}`, { defaultValue: t("errors.generic", { code: commandError }) })}
            </Callout>
          ) : (
            <p className="m-0 text-11 text-fg-3">{t("click.editsApplyOnRestart")}</p>
          )}
        </div>
      )}
      <div ref={panelRef} className="panel-scroll px-4 pb-2">
        <div ref={innerRef}>
          {profile.mode === "click" ? (
            <ClickPanel
              profile={profile}
              locale={locale}
              onChange={onEdit}
              onOpenVariation={onOpenVariation}
              onOpenPoints={onOpenPoints}
            />
          ) : profile.mode === "wiggle" ? (
            <WigglePanel
              profile={profile}
              locale={locale}
              monitorWidth={monitorWidth}
              onChange={onEdit}
              onAdjustClick={() => onEdit((p) => ({ ...p, mode: "click" }))}
            />
          ) : (
            <ComingSoon mode={profile.mode} />
          )}
        </div>
      </div>
      <Footer
        state={shortcutBroken ? "error" : engine.state}
        locale={locale}
        clicks={engine.clicks}
        elapsedMs={engine.elapsedMs ?? 0}
        countdownRemainingMs={engine.countdownRemainingMs}
        noClicks={noClicks}
        errorText={shortcutBroken ? t("status.hotkeyUnavailable", { combo: hotkeyError }) : errorText}
      />
    </div>
  );
}

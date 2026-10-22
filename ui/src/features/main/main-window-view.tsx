import { useLayoutEffect, useRef } from "react";

import { useTranslation } from "react-i18next";

import { Callout } from "../../components/callout";

import { ScanPanel } from "../scan/scan-panel";
import { useMonitorWidth } from "../wiggle/use-monitor-width";
import { WigglePanel } from "../wiggle/wiggle-panel";
import { ClickPanel } from "./click-panel";
import { Footer } from "./footer";
import { Header } from "./header";
import { MainButton, buttonStateFor } from "./main-button";
import { ModeTabs } from "./mode-tabs";
import { TitleBar } from "./title-bar";

import type { DegradedReason, EngineTick, Profile } from "../../ipc";
import type { ReactElement } from "react";

const PANEL_PADDING_BOTTOM = 8;

export interface MainWindowViewProps {
  profile: Profile;
  locale: string;
  engine: EngineTick;
  countdownTotalMs: number;
  shortcut: string;
  errorCode?: string | null;
  commandError?: string | null;
  degraded?: DegradedReason | null;
  update?: string | null;
  onOpenUpdate?: () => void;
  onEdit: (mutate: (p: Profile) => Profile) => void;
  onPrimary: () => void;
  onOpenVariation: () => void;
  onOpenPoints?: () => void;
  monitorWidth?: number;
  profiles?: readonly { id: string; name: string }[];
  onSelectProfile?: (id: string) => void;
  onOpenProfiles?: () => void;
  thumbnailUrl?: string | null;
  onSelectArea?: () => void;
  onMinimize: () => void;
  onClose: () => void;
  onDesiredHeight?: (px: number) => void;
  hotkeyError?: string | null;
  onReviewHotkeys?: () => void;
  onStartAnyway?: () => void;
  onOpenSettings?: () => void;
}

export function MainWindowView({
  profile,
  locale,
  engine,
  countdownTotalMs,
  shortcut,
  errorCode,
  commandError,
  degraded = null,
  update = null,
  onOpenUpdate,
  onEdit,
  onPrimary,
  onOpenVariation,
  onOpenPoints,
  monitorWidth: fixedMonitorWidth,
  profiles = [],
  onSelectProfile,
  onOpenProfiles,
  thumbnailUrl,
  onSelectArea,
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
  const shortcutBroken = Boolean(hotkeyError) && engine.state === "idle";
  const extraRows = `${String(active)}|${commandError ?? ""}|${String(shortcutBroken)}|${degraded ?? ""}|${update ?? ""}`;

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
    void document.fonts?.ready.then(measure);
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    for (const child of Array.from(root.children)) observer.observe(child);
    observer.observe(inner);
    return () => observer.disconnect();
  }, [onDesiredHeight, extraRows]);

  return (
    <div ref={rootRef} className="flex h-full flex-col bg-bg">
      <TitleBar onMinimize={onMinimize} onClose={onClose} />
      <Header
        profileName={profile.name}
        profiles={profiles}
        activeProfileId={profile.id}
        onSelectProfile={onSelectProfile}
        onOpenProfiles={onOpenProfiles}
        onOpenSettings={onOpenSettings}
      />
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
      {update && !active && (
        <div className="flex-none px-4 pb-3">
          <Callout tone="info">
            {t("update.available", { version: update })}{" "}
            <button type="button" className="link" onClick={onOpenUpdate}>
              {t("update.open")}
            </button>
          </Callout>
        </div>
      )}
      {degraded && (
        <div className="flex-none px-4 pb-3">
          <Callout tone={degraded === "displayChanged" ? "danger" : "warn"}>{t(`degraded.${degraded}`)}</Callout>
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
            <ScanPanel
              profile={profile}
              locale={locale}
              thumbnailUrl={thumbnailUrl ?? null}
              onChange={onEdit}
              onSelectArea={() => onSelectArea?.()}
              onAdjustClick={() => onEdit((p) => ({ ...p, mode: "click" }))}
            />
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
        blockedByAdmin={engine.blockedByAdmin}
        errorText={shortcutBroken ? t("status.hotkeyUnavailable", { combo: hotkeyError }) : errorText}
      />
    </div>
  );
}

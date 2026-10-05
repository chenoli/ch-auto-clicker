import { useState } from "react";

import { useTranslation } from "react-i18next";

import { BigNumberField } from "../../components/big-number-field";
import { RadioCard } from "../../components/radio-card";
import { Segmented } from "../../components/segmented";
import { Stepper } from "../../components/stepper";
import { Toggle } from "../../components/toggle";

import { cpsFromInterval, formatCps, formatDuration } from "../../i18n/format";

import type { MouseButton, Profile } from "../../ipc";
import type { ReactElement } from "react";

export const INTERVAL_MIN = 1;
export const INTERVAL_MAX = 60_000;
export const STOP_STEP_S = 30;
export const STOP_MIN_S = 30;
export const STOP_MAX_S = 24 * 3600;

type Edit = (mutate: (p: Profile) => Profile) => void;

interface ClickPanelProps {
  profile: Profile;
  locale: string;
  onChange: Edit;
  onOpenVariation: () => void;
  onOpenPoints?: (() => void) | undefined;
}

/** Digits only, clamped to 1–60000 ms; `null` when nothing usable was typed. */
export function parseInterval(raw: string): number | null {
  const digits = raw.replace(/\D/g, "");
  if (!digits) return null;
  return Math.min(INTERVAL_MAX, Math.max(INTERVAL_MIN, Number(digits)));
}

function IntervalRow({ profile, locale, onChange }: Pick<ClickPanelProps, "profile" | "locale" | "onChange">) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState(String(profile.click.intervalMs));
  const preview = parseInterval(draft) ?? profile.click.intervalMs;
  const commit = () => {
    const value = parseInterval(draft);
    if (value === null || value === profile.click.intervalMs) {
      setDraft(String(value ?? profile.click.intervalMs));
      return;
    }
    setDraft(String(value));
    onChange((p) => ({ ...p, click: { ...p.click, intervalMs: value } }));
  };
  return (
    <div className="settings-row min-h-15.5">
      <span className="row-label">{t("click.interval")}</span>
      <div className="flex min-w-0 items-center gap-3">
        <BigNumberField
          label={t("click.interval")}
          unit={t("units.ms")}
          value={draft}
          onChange={(raw) => setDraft(raw.replace(/\D/g, ""))}
          onCommit={commit}
        />
        <div className="flex min-w-16 flex-col items-end gap-0.75">
          <span className="font-display text-20 leading-none font-extrabold tracking-logo text-accent">
            {formatCps(cpsFromInterval(preview, profile.click.kind === "double"), locale)}
          </span>
          <span className="micro-label">{t("click.cps")}</span>
        </div>
      </div>
    </div>
  );
}

function ButtonRow({ profile, onChange }: Pick<ClickPanelProps, "profile" | "onChange">) {
  const { t } = useTranslation();
  const [menuOpen, setMenuOpen] = useState(false);
  const button = profile.click.button;
  const isSide = button === "x1" || button === "x2";
  const setButton = (b: MouseButton) => {
    setMenuOpen(false);
    onChange((p) => ({ ...p, click: { ...p.click, button: b } }));
  };
  return (
    <div className="settings-row relative min-h-11.5">
      <span className="row-label">{t("click.button")}</span>
      <Segmented
        label={t("click.button")}
        value={isSide ? "side" : button}
        onChange={(v) => (v === "side" ? setMenuOpen((open) => !open) : setButton(v))}
        options={[
          { value: "left", label: t("click.left") },
          { value: "middle", label: t("click.middle") },
          { value: "right", label: t("click.right") },
          { value: "side", label: isSide ? button.toUpperCase() : `${t("click.side")} ▾` },
        ]}
      />
      {menuOpen && (
        <div className="popover absolute top-full right-0 z-10 w-40" role="menu">
          {(["x1", "x2"] as const).map((b) => (
            <button
              key={b}
              type="button"
              role="menuitemradio"
              aria-checked={button === b}
              className="menu-item"
              onClick={() => setButton(b)}
            >
              {t(`click.${b}`)}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

/** "Parada · Parar após": shared by every mode, since the timer applies to any run. */
export function StopRow({ profile, onChange }: Pick<ClickPanelProps, "profile" | "onChange">): ReactElement {
  const { t } = useTranslation();
  const { stopAfter } = profile;
  const setStop = (seconds: number) =>
    onChange((p) => ({
      ...p,
      stopAfter: { ...p.stopAfter, seconds: Math.min(STOP_MAX_S, Math.max(STOP_MIN_S, seconds)) },
    }));
  return (
    <div className="settings-row min-h-11.5">
      <span className="row-label">{t("click.stop")}</span>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2.5">
          <Toggle
            label={t("click.stopAfter")}
            checked={stopAfter.enabled}
            onChange={(enabled) => onChange((p) => ({ ...p, stopAfter: { ...p.stopAfter, enabled } }))}
          />
          <span className="text-13 text-fg-2">{t("click.stopAfter")}</span>
        </div>
        <Stepper
          value={formatDuration(stopAfter.seconds)}
          muted={!stopAfter.enabled}
          decrementLabel={t("click.decrease")}
          incrementLabel={t("click.increase")}
          canDecrement={stopAfter.seconds > STOP_MIN_S}
          canIncrement={stopAfter.seconds < STOP_MAX_S}
          onDecrement={() => setStop(stopAfter.seconds - STOP_STEP_S)}
          onIncrement={() => setStop(stopAfter.seconds + STOP_STEP_S)}
        />
      </div>
    </div>
  );
}

export function ClickPanel({
  profile,
  locale,
  onChange,
  onOpenVariation,
  onOpenPoints,
}: ClickPanelProps): ReactElement {
  const { t } = useTranslation();
  const { click } = profile;
  const position = profile.modes.click.position;
  const variationText = click.variation.enabled
    ? t("click.variationSummary", { blocks: click.variation.blocksMs.join(" · ") })
    : t("click.variationOff", { interval: click.intervalMs });
  return (
    <div className="flex flex-col">
      <ButtonRow profile={profile} onChange={onChange} />
      <div className="settings-row min-h-11.5">
        <span className="row-label">{t("click.clickType")}</span>
        <Segmented
          label={t("click.clickType")}
          value={click.kind}
          onChange={(kind) => onChange((p) => ({ ...p, click: { ...p.click, kind } }))}
          options={[
            { value: "single", label: t("click.single") },
            { value: "double", label: t("click.double") },
          ]}
        />
      </div>
      <IntervalRow key={click.intervalMs} profile={profile} locale={locale} onChange={onChange} />
      <button type="button" className="variation-row min-h-10 text-fg-2" onClick={onOpenVariation}>
        <span className="row-label">{t("click.variation")}</span>
        <span className="text-12 font-medium tabular-nums">{variationText}</span>
        <span className="link">{t("click.edit")}</span>
      </button>
      <div className="settings-row items-start py-2.5">
        <span className="row-label pt-3">{t("click.position")}</span>
        <div className="flex flex-col gap-1.5" role="radiogroup" aria-label={t("click.position")}>
          <RadioCard
            label={t("click.cursor")}
            checked={position === "cursor"}
            onSelect={() =>
              onChange((p) => ({ ...p, modes: { ...p.modes, click: { ...p.modes.click, position: "cursor" } } }))
            }
          />
          <RadioCard
            label={t("click.points")}
            checked={position === "points"}
            onSelect={() =>
              onChange((p) => ({ ...p, modes: { ...p.modes, click: { ...p.modes.click, position: "points" } } }))
            }
            trailing={
              <button
                type="button"
                className="link"
                onClick={(e) => {
                  e.stopPropagation();
                  onOpenPoints?.();
                }}
              >
                {t("click.pointsLink", { count: profile.modes.click.points.length })}
              </button>
            }
          />
        </div>
      </div>
      <StopRow profile={profile} onChange={onChange} />
    </div>
  );
}

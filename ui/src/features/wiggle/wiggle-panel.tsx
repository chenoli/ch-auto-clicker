import { useState } from "react";

import { useTranslation } from "react-i18next";

import { Segmented } from "../../components/segmented";
import { Toggle } from "../../components/toggle";

import { StopRow } from "../main/click-panel";
import { InheritedRate } from "./inherited-rate";
import { effectiveAmplitude, formatAmplitude, parseAmplitude, previewHalfWidth, SLIDER_MAX } from "./wiggle-math";

import type { Profile } from "../../ipc";
import type { CSSProperties, ReactElement } from "react";

type Edit = (mutate: (p: Profile) => Profile) => void;

const SPEEDS = [
  { hz: 4, key: "slow" },
  { hz: 8, key: "medium" },
  { hz: 16, key: "fast" },
] as const;

interface WigglePanelProps {
  profile: Profile;
  locale: string;
  monitorWidth: number;
  onChange: Edit;
  onAdjustClick: () => void;
}

function AmplitudeRow({ profile, locale, monitorWidth, onChange }: Omit<WigglePanelProps, "onAdjustClick">) {
  const { t } = useTranslation();
  const pct = profile.modes.wiggle.amplitudePct ?? 0;
  const [draft, setDraft] = useState<string | null>(null);
  const set = (amplitudePct: number) =>
    onChange((p) => ({ ...p, modes: { ...p.modes, wiggle: { ...p.modes.wiggle, amplitudePct } } }));
  const commit = () => {
    const value = draft === null ? null : parseAmplitude(draft);
    setDraft(null);
    if (value !== null && value !== pct) set(value);
  };
  const raw = (pct / 100) * monitorWidth;
  const fill = `${(Math.min(pct, SLIDER_MAX) / SLIDER_MAX) * 100}%`;
  return (
    <div className="flex flex-col gap-3 border-t border-divider pt-3 pb-3.5">
      <div className="flex items-baseline justify-between">
        <span className="micro-label">{t("wiggle.amplitude")}</span>
        <span className="text-11 font-medium text-fg-2 tabular-nums">
          {t("wiggle.equivalent", { px: formatAmplitude(raw, locale), width: monitorWidth })}
          {raw < 1 && (
            <span className="text-accent"> {t("wiggle.effective", { px: effectiveAmplitude(pct, monitorWidth) })}</span>
          )}
        </span>
      </div>
      <div className="flex items-center gap-3.5">
        <div className="range" style={{ "--fill": fill } as CSSProperties}>
          <div className="range__track" />
          <div className="range__fill" />
          <div className="range__thumb" />
          <input
            type="range"
            min={0}
            max={SLIDER_MAX}
            step={0.01}
            value={Math.min(pct, SLIDER_MAX)}
            aria-label={t("wiggle.amplitude")}
            onChange={(e) => set(Number(e.target.value))}
          />
        </div>
        <label className="amplitude-field">
          <input
            aria-label={t("wiggle.amplitudeValue")}
            value={draft ?? formatAmplitude(pct, locale)}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commit}
            onKeyDown={(e) => {
              if (e.key === "Enter") commit();
            }}
          />
          <span>%</span>
        </label>
      </div>
    </div>
  );
}

function Preview({ pct, hz }: { pct: number; hz: number }) {
  const { t } = useTranslation();
  const half = previewHalfWidth(pct);
  return (
    <div className="flex flex-col gap-2 border-t border-divider pt-3 pb-3.5">
      <div className="flex justify-between">
        <span className="micro-label">{t("wiggle.preview")}</span>
        <span className="micro-label tracking-badge">{t("wiggle.previewScale")}</span>
      </div>
      <div className="wiggle-preview" aria-hidden="true">
        <div className="wiggle-preview__grid">
          {Array.from({ length: 10 }, (_, i) => (
            <span key={i} />
          ))}
        </div>
        <div className="wiggle-preview__band" style={{ left: `calc(50% - ${half}px)`, width: half * 2 }} />
        <div
          className="wiggle-preview__cursor"
          style={{ "--amp": `${half}px`, animationDuration: `${2 / hz}s` } as CSSProperties}
        >
          <span className="wiggle-preview__line" />
          <span className="wiggle-preview__dot" />
        </div>
      </div>
      <div className="flex justify-between text-10 font-semibold text-fg-3 tabular-nums">
        <span>0</span>
        <span>960</span>
        <span>1920 px</span>
      </div>
    </div>
  );
}

export function WigglePanel({
  profile,
  locale,
  monitorWidth,
  onChange,
  onAdjustClick,
}: WigglePanelProps): ReactElement {
  const { t } = useTranslation();
  const wiggle = profile.modes.wiggle;
  const setWiggle = (patch: Partial<Profile["modes"]["wiggle"]>) =>
    onChange((p) => ({ ...p, modes: { ...p.modes, wiggle: { ...p.modes.wiggle, ...patch } } }));

  return (
    <div className="flex flex-col">
      <AmplitudeRow profile={profile} locale={locale} monitorWidth={monitorWidth} onChange={onChange} />
      <Preview pct={wiggle.amplitudePct ?? 0} hz={wiggle.speedHz} />
      <div className="settings-row min-h-12.5 grid-cols-[84px_1fr_auto] gap-3">
        <span className="row-label">{t("wiggle.speed")}</span>
        <Segmented
          label={t("wiggle.speed")}
          value={String(wiggle.speedHz)}
          onChange={(v) => setWiggle({ speedHz: Number(v) })}
          options={SPEEDS.map((s) => ({ value: String(s.hz), label: t(`wiggle.${s.key}`) }))}
        />
        <span className="min-w-12 text-right font-display text-18 font-extrabold text-fg">
          {wiggle.speedHz}
          <span className="font-ui text-11 font-semibold text-fg-3"> Hz</span>
        </span>
      </div>
      <div className="settings-row items-start py-3">
        <span className="row-label pt-1">{t("wiggle.clicks")}</span>
        <div className="flex min-w-0 flex-col gap-2">
          <div className="flex items-center justify-between">
            <span className="text-13 text-fg">{t("wiggle.clickWhile")}</span>
            <Toggle
              label={t("wiggle.clickWhile")}
              checked={wiggle.clickWhileWiggling}
              onChange={(clickWhileWiggling) => setWiggle({ clickWhileWiggling })}
            />
          </div>
          {wiggle.clickWhileWiggling && <InheritedRate profile={profile} locale={locale} onAdjust={onAdjustClick} />}
        </div>
      </div>
      <StopRow profile={profile} onChange={onChange} />
    </div>
  );
}

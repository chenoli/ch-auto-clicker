import { useTranslation } from "react-i18next";

import { Segmented } from "../../components/segmented";
import { Stepper } from "../../components/stepper";
import { Toggle } from "../../components/toggle";

import { formatOneDecimal } from "../../i18n/format";

import { StopRow } from "../main/click-panel";
import { InheritedRate } from "../wiggle/inherited-rate";
import {
  SPACING_MAX,
  SPACING_MIN,
  SPACING_STEP,
  SPEEDS,
  THUMB,
  clampSpacing,
  scanCycleSecs,
  scanRows,
  thumbPolyline,
} from "./scan-math";

import type { Profile, ScanAction } from "../../ipc";
import type { ReactElement } from "react";

type Edit = (mutate: (p: Profile) => Profile) => void;

interface ScanPanelProps {
  profile: Profile;
  locale: string;
  thumbnailUrl: string | null;
  onChange: Edit;
  onSelectArea: () => void;
  onAdjustClick: () => void;
}

export function ScanPanel({
  profile,
  locale,
  thumbnailUrl,
  onChange,
  onSelectArea,
  onAdjustClick,
}: ScanPanelProps): ReactElement {
  const { t } = useTranslation();
  const scan = profile.modes.scan;
  const area = scan.area;
  const setScan = (patch: Partial<Profile["modes"]["scan"]>) =>
    onChange((p) => ({ ...p, modes: { ...p.modes, scan: { ...p.modes.scan, ...patch } } }));
  const setSpacing = (n: number) => setScan({ lineSpacingPx: clampSpacing(n) });

  return (
    <div className="flex flex-col">
      <div className="flex min-h-12 items-center justify-between border-t border-divider">
        <button type="button" className="select-area" onClick={onSelectArea}>
          <span className="select-area__icon" aria-hidden="true" />
          {t("scan.selectArea")}
        </button>
        <span className="text-11 font-medium text-fg-2 tabular-nums">
          {area ? t("scan.areaMeta", { w: area.w, h: area.h, x: area.x, y: area.y }) : t("scan.noArea")}
        </span>
      </div>
      <div className="scan-thumb mb-4">
        {thumbnailUrl && area && <img src={thumbnailUrl} alt="" className="scan-thumb__image" />}
        {!thumbnailUrl && <span className="scan-thumb__placeholder">{t("scan.thumbPlaceholder")}</span>}
        {area ? (
          <>
            <svg width={THUMB.width} height={THUMB.height} className="absolute inset-0" aria-hidden="true">
              <polyline
                points={thumbPolyline(area, scan.lineSpacingPx)}
                fill="none"
                stroke="var(--accent)"
                strokeWidth="1.25"
                strokeLinejoin="round"
              />
            </svg>
            <span className="scan-thumb__start" />
            <span className="scan-thumb__badge">
              {t("scan.badge", {
                rows: scanRows(area, scan.lineSpacingPx),
                cycle: formatOneDecimal(scanCycleSecs(area, scan.lineSpacingPx, scan.speedPxS, scan.loop), locale),
              })}
            </span>
          </>
        ) : (
          <span className="scan-thumb__empty">{t("scan.selectHint")}</span>
        )}
      </div>
      <div className="settings-row min-h-13.5">
        <span className="row-label">{t("scan.lines")}</span>
        <div className="flex items-center justify-between">
          <span className="text-12 text-fg-2">{t("scan.spacing")}</span>
          <Stepper
            value={`${scan.lineSpacingPx} px`}
            decrementLabel={t("scan.decrease")}
            incrementLabel={t("scan.increase")}
            canDecrement={scan.lineSpacingPx > SPACING_MIN}
            canIncrement={scan.lineSpacingPx < SPACING_MAX}
            onDecrement={() => setSpacing(scan.lineSpacingPx - SPACING_STEP)}
            onIncrement={() => setSpacing(scan.lineSpacingPx + SPACING_STEP)}
          />
        </div>
      </div>
      <div className="settings-row min-h-13.5">
        <span className="row-label">{t("scan.speed")}</span>
        <Segmented
          label={t("scan.speed")}
          variant="compact"
          value={String(scan.speedPxS)}
          onChange={(v) => setScan({ speedPxS: Number(v) })}
          options={SPEEDS.map((s) => ({ value: String(s), label: `${s} px/s` }))}
        />
      </div>
      <div className="settings-row min-h-13.5">
        <span className="row-label">{t("scan.action")}</span>
        <Segmented<ScanAction>
          label={t("scan.action")}
          variant="compact"
          value={scan.action}
          onChange={(action) => setScan({ action })}
          options={[
            { value: "move", label: t("scan.moveOnly") },
            { value: "move_click", label: t("scan.moveClick") },
          ]}
        />
      </div>
      {scan.action === "move_click" && (
        <div className="grid grid-cols-[84px_1fr] pb-3.5">
          <span />
          <InheritedRate profile={profile} locale={locale} onAdjust={onAdjustClick} />
        </div>
      )}
      <div className="settings-row min-h-13.5">
        <span className="row-label">{t("scan.loop")}</span>
        <div className="flex items-center justify-between">
          <span className="text-12 text-fg-2">{t("scan.loopHint")}</span>
          <Toggle label={t("scan.loopHint")} checked={scan.loop} onChange={(loop) => setScan({ loop })} />
        </div>
      </div>
      <StopRow profile={profile} onChange={onChange} />
    </div>
  );
}

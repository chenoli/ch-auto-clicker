import { useTranslation } from "react-i18next";

import { countdownSeconds, formatCount, formatDuration } from "../../i18n/format";

import type { EngineState } from "../../ipc";
import type { ReactElement } from "react";

export interface HudViewProps {
  state: EngineState;
  locale: string;
  profileName: string;
  clicks: number;
  countdownRemainingMs: number | null;
  remainingMs: number | null;
  startStop: string;
  pauseResume: string;
  hover?: boolean;
  draggable?: boolean;
  onDragStart?: () => void;
}

export function HudView(props: HudViewProps): ReactElement | null {
  const { t } = useTranslation();
  const { state, locale } = props;
  if (state !== "countdown" && state !== "running" && state !== "paused") return null;
  const remaining = props.remainingMs == null ? null : formatDuration(props.remainingMs / 1000).replace(/^00:/, "");
  const chip =
    state === "countdown"
      ? { key: props.startStop, verb: t("button.cancels") }
      : state === "running"
        ? { key: props.startStop, verb: t("button.stops") }
        : { key: props.pauseResume, verb: t("button.resumes") };
  return (
    <div className="flex flex-col items-end gap-1.5">
      <div
        className="hud-pill"
        data-state={state}
        data-hover={props.hover ?? false}
        data-draggable={props.draggable ?? false}
        role="status"
        onPointerDown={() => props.draggable && props.onDragStart?.()}
      >
        {state === "paused" ? (
          <span className="hud-pill__bars" aria-hidden="true">
            <span />
            <span />
          </span>
        ) : (
          <span className="hud-pill__dot" aria-hidden="true" />
        )}
        <span className="hud-pill__state">{t(`hud.${state}`)}</span>
        {state === "countdown" ? (
          <>
            <span className="font-display text-20 leading-none font-extrabold text-fg">
              {countdownSeconds(props.countdownRemainingMs ?? 0)}
            </span>
            <span className="hud-pill__sep" />
            <span className="max-w-40 truncate text-12 font-medium text-fg-2">{props.profileName}</span>
          </>
        ) : (
          <>
            <span className="flex items-baseline gap-1.25">
              <span className="font-display text-17 font-extrabold text-fg tabular-nums">
                {formatCount(props.clicks, locale)}
              </span>
              <span className="text-10 text-fg-2">{t("hud.clicks")}</span>
            </span>
            {remaining && (
              <>
                <span className="hud-pill__sep" />
                <span className="flex items-baseline gap-1.25">
                  <span className="hud-pill__label">{t("hud.remaining")}</span>
                  <span className="hud-pill__time">{remaining}</span>
                </span>
              </>
            )}
          </>
        )}
        <span className="hud-pill__chip">
          <span className="font-bold text-fg">{chip.key}</span>
          {chip.verb}
        </span>
      </div>
      <span className="hud-hint">{t("hud.hint")}</span>
    </div>
  );
}

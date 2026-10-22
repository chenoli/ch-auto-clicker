import { useTranslation } from "react-i18next";

import { formatCount, formatElapsed, formatOneDecimal } from "../../i18n/format";

import type { EngineState } from "../../ipc";
import type { ReactElement } from "react";

interface FooterProps {
  state: EngineState;
  locale: string;
  clicks: number;
  elapsedMs: number;
  countdownRemainingMs: number | null;
  noClicks?: boolean;
  blockedByAdmin?: boolean;
  errorText?: string;
}

const TONES: Record<EngineState, string | undefined> = {
  idle: undefined,
  countdown: "warn",
  running: "accent",
  paused: undefined,
  stopping: undefined,
  error: "danger",
};

export function Footer({
  state,
  locale,
  clicks,
  elapsedMs,
  countdownRemainingMs,
  noClicks = false,
  blockedByAdmin = false,
  errorText = "",
}: FooterProps): ReactElement {
  const { t } = useTranslation();
  const blocked = state === "running" && blockedByAdmin;
  const status = blocked
    ? t("status.adminBlocked")
    : state === "countdown"
      ? t("status.countdown", { seconds: formatOneDecimal((countdownRemainingMs ?? 0) / 1000, locale) })
      : state === "error"
        ? errorText
        : t(`status.${state}`);
  return (
    <div className="flex h-10 flex-none items-center gap-4 border-t border-divider px-4 text-11">
      <div className="flex flex-1 items-center gap-2" data-tone={blocked ? "warn" : TONES[state]}>
        <span className="status-dot" />
        <span className="status-text">{status}</span>
      </div>
      <div className="flex items-baseline gap-1.5">
        <span className="micro-label">{t("status.clicks")}</span>
        <span className="font-semibold text-fg tabular-nums">{noClicks ? "—" : formatCount(clicks, locale)}</span>
      </div>
      <div className="flex items-baseline gap-1.5">
        <span className="micro-label">{t("status.time")}</span>
        <span className="font-semibold text-fg tabular-nums">{formatElapsed(elapsedMs)}</span>
      </div>
    </div>
  );
}

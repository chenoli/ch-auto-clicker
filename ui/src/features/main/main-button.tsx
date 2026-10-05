import { useTranslation } from "react-i18next";

import { Kbd } from "../../components/kbd";

import { countdownSeconds } from "../../i18n/format";

import type { EngineState } from "../../ipc";
import type { ReactElement } from "react";

/** The visual states of Tela 10; "error" covers engine errors (hotkey conflicts arrive in Phase 4). */
export type MainButtonState = "idle" | "countdown" | "running" | "paused" | "error";

export function buttonStateFor(engine: EngineState): MainButtonState {
  switch (engine) {
    case "countdown":
    case "running":
    case "paused":
    case "error":
      return engine;
    case "stopping":
      return "running";
    case "idle":
      return "idle";
  }
}

interface MainButtonProps {
  state: MainButtonState;
  shortcut: string;
  countdownRemainingMs?: number | null;
  countdownTotalMs?: number;
  /** Kicker for the error state, e.g. "Entrada bloqueada". */
  errorKicker?: string;
  onPress: () => void;
}

export function MainButton({
  state,
  shortcut,
  countdownRemainingMs,
  countdownTotalMs = 0,
  errorKicker = "",
  onPress,
}: MainButtonProps): ReactElement {
  const { t } = useTranslation();
  const remaining = countdownRemainingMs ?? countdownTotalMs;
  const copy: Record<MainButtonState, { kicker: string; label: string; hint: string }> = {
    idle: { kicker: t("button.idle.kicker"), label: t("button.idle.label"), hint: t("button.shortcut") },
    countdown: {
      kicker: t("button.countdown.kicker"),
      label: String(countdownSeconds(remaining)),
      hint: t("button.cancels"),
    },
    running: { kicker: t("button.running.kicker"), label: t("button.running.label"), hint: t("button.stops") },
    paused: { kicker: t("button.paused.kicker"), label: t("button.paused.label"), hint: t("button.resumes") },
    error: { kicker: errorKicker, label: t("button.error.label"), hint: t("button.shortcut") },
  };
  const { kicker, label, hint } = copy[state];
  const progress =
    state === "countdown" && countdownTotalMs > 0 ? Math.min(100, 100 - (remaining / countdownTotalMs) * 100) : 0;

  return (
    <div className="flex-none px-4 pt-3 pb-3.5">
      <button type="button" className="main-button" data-state={state} onClick={onPress}>
        <span className="flex flex-col gap-1">
          <span className="main-button__kicker">{kicker}</span>
          <span className="main-button__label">{label}</span>
        </span>
        <span className="flex items-center gap-2.5">
          <span className="main-button__hint">{hint}</span>
          <Kbd size="large">{shortcut}</Kbd>
        </span>
        <span className="main-button__progress" style={{ width: `${progress}%` }} />
      </button>
    </div>
  );
}

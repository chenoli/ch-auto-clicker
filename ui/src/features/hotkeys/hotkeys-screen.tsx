import { useTranslation } from "react-i18next";

import { Kbd } from "../../components/kbd";

import { TitleBar } from "../main/title-bar";
import { SubScreenHeader } from "../shell/sub-screen-header";
import { hotkeyKeys, hotkeyLabel } from "./format-hotkey";

import type { Hotkey, HotkeyAction, HotkeyBindings, HotkeyStatus, HotkeyTarget, Modifier } from "../../ipc";
import type { ReactElement } from "react";

export const ACTIONS: readonly HotkeyAction[] = [
  "startStop",
  "pauseResume",
  "capturePoint",
  "nextProfile",
  "emergencyStop",
];

export interface PendingConflict {
  target: HotkeyTarget;
  combo: Hotkey;
  withName: string;
}

export interface HotkeysScreenProps {
  bindings: HotkeyBindings;
  statuses: readonly HotkeyStatus[];
  recording: HotkeyAction | null;
  heldMods: readonly Modifier[];
  conflict: PendingConflict | null;
  onRecord: (action: HotkeyAction) => void;
  onReplace: () => void;
  onKeep: () => void;
  onReset: () => void;
  onRetry: () => void;
  onBack: () => void;
  onMinimize: () => void;
  onClose: () => void;
}

const MOD_LABELS: Record<Modifier, string> = { ctrl: "Ctrl", alt: "Alt", shift: "Shift", win: "Win" };

function bindingOf(bindings: HotkeyBindings, action: HotkeyAction): Hotkey | null {
  return bindings[action];
}

export function HotkeysScreen({
  bindings,
  statuses,
  recording,
  heldMods,
  conflict,
  onRecord,
  onReplace,
  onKeep,
  onReset,
  onRetry,
  onBack,
  onMinimize,
  onClose,
}: HotkeysScreenProps): ReactElement {
  const { t } = useTranslation();

  return (
    <div className="flex h-full flex-col bg-bg">
      <TitleBar onMinimize={onMinimize} onClose={onClose} />
      <SubScreenHeader
        kicker={t("hotkeys.kicker")}
        title={t("hotkeys.title")}
        onBack={onBack}
        action={
          <button type="button" className="button--text text-12 font-semibold" onClick={onReset}>
            {t("hotkeys.reset")}
          </button>
        }
      />
      <div className="min-h-0 flex-1 overflow-auto px-4">
        {ACTIONS.map((action) => {
          const combo = bindingOf(bindings, action);
          const status = statuses.find((s) => s.target.kind === "action" && s.target.id === action);
          const taken = status ? !status.ok : false;
          const isRecording = recording === action;
          const isConflict = conflict?.target.kind === "action" && conflict.target.id === action;
          const shown = isConflict ? conflict.combo : combo;
          const state = isRecording ? "recording" : isConflict || taken ? "conflict" : "normal";
          return (
            <div key={action} className="flex flex-col gap-2.5 border-t border-divider py-3.5">
              <div className="flex items-center gap-3">
                <div className="flex min-w-0 flex-1 flex-col gap-0.75">
                  <span className="text-13 font-semibold text-fg">{t(`hotkeys.actions.${action}.name`)}</span>
                  <span className="text-11 text-fg-3">{t(`hotkeys.actions.${action}.desc`)}</span>
                </div>
                <button
                  type="button"
                  className="capture-field"
                  data-state={state}
                  aria-label={`${t(`hotkeys.actions.${action}.name`)}: ${hotkeyLabel(combo)}`}
                  onClick={() => onRecord(action)}
                >
                  {isRecording ? (
                    heldMods.length ? (
                      heldMods.map((m) => <Kbd key={m}>{MOD_LABELS[m]}</Kbd>)
                    ) : (
                      <span className="capture-field__recording">
                        <span className="pulse-dot" />
                        {t("hotkeys.recording")}
                      </span>
                    )
                  ) : shown ? (
                    hotkeyKeys(shown).map((k) => <Kbd key={k}>{k}</Kbd>)
                  ) : (
                    <span className="text-13 text-fg-3">—</span>
                  )}
                </button>
              </div>
              {isConflict && (
                <div className="conflict-box" role="alert">
                  <span>{t("hotkeys.conflict", { combo: hotkeyLabel(conflict.combo), name: conflict.withName })}</span>
                  <div className="flex gap-1.5">
                    <button type="button" className="small-action small-action--danger" onClick={onReplace}>
                      {t("hotkeys.replace")}
                    </button>
                    <button type="button" className="small-action" onClick={onKeep}>
                      {t("hotkeys.keep")}
                    </button>
                  </div>
                </div>
              )}
              {taken && !isConflict && !isRecording && (
                <div className="conflict-box" role="alert">
                  <span>{t("hotkeys.taken")}</span>
                  <div className="flex gap-1.5">
                    <button type="button" className="small-action" onClick={onRetry}>
                      {t("hotkeys.retry")}
                    </button>
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
      <div className="flex h-11 flex-none items-center border-t border-divider px-4 text-11 leading-body text-fg-3">
        {t("hotkeys.footer")}
      </div>
    </div>
  );
}

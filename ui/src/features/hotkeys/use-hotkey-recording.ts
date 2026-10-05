import { useCallback, useEffect, useState } from "react";

import { useTranslation } from "react-i18next";

import { commands, events, inTauri } from "../../ipc";

import { setCommandError, useAppStore } from "../../store/app-store";

import type { Hotkey, HotkeyAction, HotkeyTarget, Modifier } from "../../ipc";
import type { PendingConflict } from "./hotkeys-screen";

export interface HotkeyRecordingApi {
  recording: HotkeyAction | null;
  heldMods: Modifier[];
  conflict: PendingConflict | null;
  record: (action: HotkeyAction) => void;
  cancel: () => void;
  replace: () => void;
  keep: () => void;
}

/** Drives the capture field: Rust records through its keyboard hook and reports each step. */
export function useHotkeyRecording(): HotkeyRecordingApi {
  const { t } = useTranslation();
  const profiles = useAppStore((s) => s.profiles);
  const [recording, setRecording] = useState<HotkeyAction | null>(null);
  const [heldMods, setHeldMods] = useState<Modifier[]>([]);
  const [conflict, setConflict] = useState<PendingConflict | null>(null);

  const nameOf = useCallback(
    (target: HotkeyTarget) =>
      target.kind === "action"
        ? t(`hotkeys.actions.${target.id}.name`)
        : t("hotkeys.profileShortcut", { name: profiles.find((p) => p.id === target.id)?.name ?? target.id }),
    [profiles, t],
  );

  const save = useCallback(
    async (action: HotkeyAction, combo: Hotkey, replace: boolean) => {
      const result = await commands.hotkeySet({ kind: "action", id: action }, combo, replace);
      if (result.status === "error") {
        setCommandError(result.error.code);
        setConflict(null);
        return;
      }
      if (result.data.result === "conflict") {
        setConflict({ action, combo, withName: nameOf(result.data.with) });
      } else {
        setConflict(null);
      }
    },
    [nameOf],
  );

  useEffect(() => {
    if (!inTauri || !recording) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void events.hotkeyRecording
      .listen(({ payload }) => {
        if (payload.cancelled) {
          setRecording(null);
          setHeldMods([]);
        } else if (payload.combo) {
          setRecording(null);
          setHeldMods([]);
          void save(recording, payload.combo, false);
        } else {
          setHeldMods(payload.mods);
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [recording, save]);

  const record = useCallback((action: HotkeyAction) => {
    setConflict(null);
    setHeldMods([]);
    setRecording(action);
    if (inTauri) void commands.hotkeyBeginRecord();
  }, []);

  const cancel = useCallback(() => {
    setRecording((current) => {
      if (current && inTauri) void commands.hotkeyCancelRecord();
      return null;
    });
    setHeldMods([]);
  }, []);

  const replace = useCallback(() => {
    if (conflict) void save(conflict.action, conflict.combo, true);
  }, [conflict, save]);

  const keep = useCallback(() => setConflict(null), []);

  return { recording, heldMods, conflict, record, cancel, replace, keep };
}

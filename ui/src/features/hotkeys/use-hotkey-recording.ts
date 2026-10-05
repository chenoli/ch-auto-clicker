import { useCallback, useEffect, useState } from "react";

import { useTranslation } from "react-i18next";

import { commands, events, inTauri } from "../../ipc";

import { setCommandError, useAppStore } from "../../store/app-store";

import type { Hotkey, HotkeyAction, HotkeyTarget, Modifier } from "../../ipc";
import type { PendingConflict } from "./hotkeys-screen";

export interface HotkeyRecordingApi {
  recording: HotkeyAction | null;
  recordingProfile: string | null;
  heldMods: Modifier[];
  conflict: PendingConflict | null;
  record: (action: HotkeyAction) => void;
  recordProfile: (id: string) => void;
  clearProfile: (id: string) => void;
  cancel: () => void;
  replace: () => void;
  keep: () => void;
}

export function useHotkeyRecording(): HotkeyRecordingApi {
  const { t } = useTranslation();
  const profiles = useAppStore((s) => s.profiles);
  const [recording, setRecording] = useState<HotkeyTarget | null>(null);
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
    async (target: HotkeyTarget, combo: Hotkey | null, replace: boolean) => {
      const result = await commands.hotkeySet(target, combo, replace);
      if (result.status === "error") {
        setCommandError(result.error.code);
        setConflict(null);
        return;
      }
      if (result.data.result === "conflict" && combo) {
        setConflict({ target, combo, withName: nameOf(result.data.with) });
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

  const start = useCallback((target: HotkeyTarget) => {
    setConflict(null);
    setHeldMods([]);
    setRecording(target);
    if (inTauri) void commands.hotkeyBeginRecord();
  }, []);
  const record = useCallback((action: HotkeyAction) => start({ kind: "action", id: action }), [start]);
  const recordProfile = useCallback((id: string) => start({ kind: "profile", id }), [start]);
  const clearProfile = useCallback((id: string) => void save({ kind: "profile", id }, null, false), [save]);

  const cancel = useCallback(() => {
    setRecording((current) => {
      if (current && inTauri) void commands.hotkeyCancelRecord();
      return null;
    });
    setHeldMods([]);
  }, []);

  const replace = useCallback(() => {
    if (conflict) void save(conflict.target, conflict.combo, true);
  }, [conflict, save]);

  const keep = useCallback(() => setConflict(null), []);

  return {
    recording: recording?.kind === "action" ? recording.id : null,
    recordingProfile: recording?.kind === "profile" ? recording.id : null,
    heldMods,
    conflict,
    record,
    recordProfile,
    clearProfile,
    cancel,
    replace,
    keep,
  };
}

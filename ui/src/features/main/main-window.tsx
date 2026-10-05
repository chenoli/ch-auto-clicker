import { useCallback, useEffect, useState } from "react";

import { getCurrentWindow } from "@tauri-apps/api/window";

import { commands, inTauri } from "../../ipc";

import { selectActiveProfile, setCommandError, useAppStore } from "../../store/app-store";
import { connectStore } from "../../store/connect";
import { primaryAction } from "../../store/engine-actions";
import { editProfile } from "../../store/profile-sync";

import { hotkeyLabel } from "../hotkeys/format-hotkey";
import { ConnectFailure } from "../shell/connect-failure";
import { MainWindowView } from "./main-window-view";

import type { Profile } from "../../ipc";
import type { ReactElement } from "react";

/** The main window (Tela 01), connected to the Rust state. */
export function MainWindow(): ReactElement | null {
  const ready = useAppStore((s) => s.ready);
  const profile = useAppStore(selectActiveProfile);
  const settings = useAppStore((s) => s.settings);
  const engine = useAppStore((s) => s.engine);
  const countdownTotalMs = useAppStore((s) => s.countdownTotalMs);
  const lastStatus = useAppStore((s) => s.lastStatus);
  const commandError = useAppStore((s) => s.commandError);

  const [connectError, setConnectError] = useState<string | null>(null);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    connectStore()
      .then((d) => (dispose = d))
      .catch((e: unknown) => setConnectError(String(e)));
    return () => dispose?.();
  }, []);

  const onEdit = useCallback(
    (mutate: (p: Profile) => Profile) => {
      if (profile) editProfile(profile.id, mutate);
    },
    [profile],
  );

  const onDesiredHeight = useCallback((px: number) => {
    if (inTauri) void commands.windowSetContentHeight(px);
  }, []);

  if (connectError) return <ConnectFailure detail={connectError} />;
  if (!ready || !profile || !settings) return null;

  return (
    <MainWindowView
      profile={profile}
      locale={settings.locale}
      engine={engine}
      countdownTotalMs={countdownTotalMs}
      shortcut={hotkeyLabel(settings.hotkeys.startStop)}
      errorCode={engine.state === "error" ? (lastStatus?.errorCode ?? null) : null}
      commandError={commandError}
      onEdit={onEdit}
      onPrimary={() => void primaryAction(engine.state)}
      onOpenVariation={() => {
        setCommandError(null);
        void commands.windowOpenVariation();
      }}
      onMinimize={() => void getCurrentWindow().minimize()}
      onClose={() => void getCurrentWindow().close()}
      onDesiredHeight={onDesiredHeight}
    />
  );
}

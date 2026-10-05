import { useCallback, useEffect, useState } from "react";

import { getCurrentWindow } from "@tauri-apps/api/window";

import { commands, events, inTauri } from "../../ipc";

import { selectActiveProfile, setCommandError, useAppStore } from "../../store/app-store";
import { connectStore } from "../../store/connect";
import { primaryAction } from "../../store/engine-actions";
import { editProfile } from "../../store/profile-sync";

import { hotkeyLabel } from "../hotkeys/format-hotkey";
import { HotkeysScreen } from "../hotkeys/hotkeys-screen";
import { useHotkeyRecording } from "../hotkeys/use-hotkey-recording";
import { chcapUrl } from "../overlay/overlay-keys";
import { PointsRoute } from "../points/points-route";
import { ProfilesRoute } from "../profiles/profiles-route";
import { SettingsRoute } from "../settings/settings-route";
import { ConnectFailure } from "../shell/connect-failure";
import { ThemesRoute } from "../themes/themes-route";
import { AntiCheatDialog } from "./anti-cheat-dialog";
import { MainWindowView } from "./main-window-view";

import type { Profile } from "../../ipc";
import type { ReactElement } from "react";

type Screen = "main" | "settings" | "hotkeys" | "points" | "profiles" | "themes";

const MIN_HEIGHT = 640;
/** Rust refuses the first start until the anti-cheat notice is accepted (T11.3). */
const ANTI_CHEAT_ACK = "ANTI_CHEAT_ACK";

/** The scan thumbnail; `updatedAt` busts the webview cache when a new area is saved. */
function thumbnailUrl(profile: Profile): string {
  return `${chcapUrl(`thumb/${profile.id}.bmp`)}?v=${encodeURIComponent(profile.updatedAt)}`;
}

/** The main window (Tela 01) and its sub-screens, connected to the Rust state. */
export function MainWindow(): ReactElement | null {
  const ready = useAppStore((s) => s.ready);
  const profile = useAppStore(selectActiveProfile);
  const settings = useAppStore((s) => s.settings);
  const engine = useAppStore((s) => s.engine);
  const countdownTotalMs = useAppStore((s) => s.countdownTotalMs);
  const lastStatus = useAppStore((s) => s.lastStatus);
  const commandError = useAppStore((s) => s.commandError);
  const hotkeys = useAppStore((s) => s.hotkeys);
  const profiles = useAppStore((s) => s.profiles);
  const [screen, setScreen] = useState<Screen>("main");
  const recordingApi = useHotkeyRecording();
  const { cancel: cancelRecording } = recordingApi;
  const [connectError, setConnectError] = useState<string | null>(null);

  // "Configurações" in the tray menu opens that screen here.
  useEffect(() => {
    if (!inTauri) return;
    const unlisten = events.navigateRequested.listen((e) => {
      if (e.payload.screen === "settings") setScreen("settings");
    });
    return () => void unlisten.then((f) => f());
  }, []);

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

  const go = useCallback(
    (next: Screen) => {
      cancelRecording();
      setScreen(next);
      // Sub-screens scroll inside the minimum height; the main screen re-measures on its own.
      if (next !== "main") onDesiredHeight(MIN_HEIGHT);
    },
    [cancelRecording, onDesiredHeight],
  );

  if (connectError) return <ConnectFailure detail={connectError} />;
  if (!ready || !profile || !settings) return null;

  const windowActions = {
    onMinimize: () => void getCurrentWindow().minimize(),
    onClose: () => void getCurrentWindow().close(),
  };

  if (screen === "settings") {
    return (
      <SettingsRoute
        settings={settings}
        onOpenThemes={() => go("themes")}
        onOpenHotkeys={() => go("hotkeys")}
        onBack={() => go("main")}
        {...windowActions}
      />
    );
  }

  if (screen === "themes") {
    return <ThemesRoute settings={settings} onBack={() => go("settings")} {...windowActions} />;
  }

  if (screen === "hotkeys") {
    return (
      <HotkeysScreen
        bindings={settings.hotkeys}
        statuses={hotkeys}
        recording={recordingApi.recording}
        heldMods={recordingApi.heldMods}
        conflict={recordingApi.conflict}
        onRecord={recordingApi.record}
        onReplace={recordingApi.replace}
        onKeep={recordingApi.keep}
        onReset={() => void commands.hotkeyResetDefaults()}
        onRetry={() => void commands.hotkeyRetry()}
        onBack={() => go("settings")}
        {...windowActions}
      />
    );
  }

  if (screen === "profiles") {
    return <ProfilesRoute recording={recordingApi} onBack={() => go("main")} {...windowActions} />;
  }

  if (screen === "points") {
    return (
      <PointsRoute
        profile={profile}
        captureHotkey={settings.hotkeys.capturePoint}
        onBack={() => go("main")}
        {...windowActions}
      />
    );
  }

  const startStatus = hotkeys.find((h) => h.target.kind === "action" && h.target.id === "startStop");
  return (
    <>
      <MainWindowView
        profile={profile}
        locale={settings.locale}
        engine={engine}
        countdownTotalMs={countdownTotalMs}
        shortcut={hotkeyLabel(settings.hotkeys.startStop)}
        errorCode={engine.state === "error" ? (lastStatus?.errorCode ?? null) : null}
        commandError={commandError}
        hotkeyError={startStatus && !startStatus.ok ? hotkeyLabel(startStatus.combo) : null}
        onEdit={onEdit}
        onPrimary={() => void primaryAction(engine.state)}
        onReviewHotkeys={() => go("hotkeys")}
        onStartAnyway={() => void primaryAction("idle")}
        onOpenSettings={() => go("settings")}
        onOpenVariation={() => {
          setCommandError(null);
          void commands.windowOpenVariation();
        }}
        onOpenPoints={() => go("points")}
        profiles={profiles}
        onSelectProfile={(id) => {
          setCommandError(null);
          void commands.profileActivate(id).then((r) => {
            if (r.status === "error") setCommandError(r.error.code);
          });
        }}
        onOpenProfiles={() => go("profiles")}
        thumbnailUrl={profile.modes.scan.area && inTauri ? thumbnailUrl(profile) : null}
        onSelectArea={() => {
          setCommandError(null);
          void commands.overlayPickArea().then((r) => {
            if (r.status === "error") setCommandError(r.error.code);
          });
        }}
        onDesiredHeight={onDesiredHeight}
        {...windowActions}
      />
      {commandError === ANTI_CHEAT_ACK && (
        <AntiCheatDialog
          onAccept={() => {
            setCommandError(null);
            void commands.settingsAcknowledgeAntiCheat().then(() => primaryAction("idle"));
          }}
          onCancel={() => setCommandError(null)}
        />
      )}
    </>
  );
}

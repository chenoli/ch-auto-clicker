import { commands, events, inTauri } from "../ipc";

import {
  applyCapabilities,
  applyDegraded,
  applyHotkeys,
  applySettings,
  applyUpdate,
  applyStatus,
  applyTick,
  hydrate,
  setCommandError,
} from "./app-store";
import { applyProfilesChanged, rememberServerVersions } from "./profile-sync";

/** Loads the initial state and keeps the store in sync with Rust. Returns a disposer. */
export async function connectStore(): Promise<() => void> {
  if (!inTauri) return () => {};
  const unlisten = await Promise.all([
    events.engineTick.listen((e) => applyTick(e.payload)),
    events.engineStatus.listen((e) => applyStatus(e.payload)),
    events.settingsChanged.listen((e) => applySettings(e.payload)),
    events.profilesChanged.listen((e) => applyProfilesChanged(e.payload)),
    events.capabilities.listen((e) => applyCapabilities(e.payload)),
    events.engineDegraded.listen((e) => applyDegraded(e.payload)),
    events.updateAvailable.listen((e) => applyUpdate(e.payload.version)),
    events.hotkeyStatusChanged.listen((e) => applyHotkeys(e.payload)),
    events.hotkeyActionFailed.listen((e) => setCommandError(e.payload.code)),
  ]);
  const bootstrap = await commands.appBootstrap();
  rememberServerVersions(bootstrap.profiles);
  applyProfilesChanged({ profiles: bootstrap.profiles, activeProfileId: bootstrap.settings.activeProfileId });
  hydrate(bootstrap);
  return () => unlisten.forEach((fn) => fn());
}

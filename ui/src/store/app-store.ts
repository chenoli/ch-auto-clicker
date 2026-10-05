import { create } from "zustand";

import type { Bootstrap, Capabilities, EngineStatus, EngineTick, HotkeyStatus, Profile, Settings } from "../ipc";

export const IDLE_TICK: EngineTick = {
  state: "idle",
  fires: 0,
  clicks: 0,
  skippedSelf: 0,
  missed: 0,
  elapsedMs: 0,
  countdownRemainingMs: null,
  remainingMs: null,
};

/** Mirror of the Rust state (the source of truth), fed by bootstrap and events. */
export interface AppState {
  ready: boolean;
  version: string;
  settings: Settings | null;
  profiles: Profile[];
  engine: EngineTick;
  lastStatus: EngineStatus | null;
  /** Length of the current countdown, for the progress bar on the main button. */
  countdownTotalMs: number;
  capabilities: Capabilities | null;
  /** Registration result of every global shortcut. */
  hotkeys: HotkeyStatus[];
  /** Error code from the last failed command, shown until the next action. */
  commandError: string | null;
}

export const useAppStore = create<AppState>(() => ({
  ready: false,
  version: "",
  settings: null,
  profiles: [],
  engine: IDLE_TICK,
  lastStatus: null,
  countdownTotalMs: 0,
  capabilities: null,
  hotkeys: [],
  commandError: null,
}));

export function hydrate(b: Bootstrap): void {
  useAppStore.setState({
    ready: true,
    version: b.version,
    settings: b.settings,
    profiles: b.profiles,
    engine: b.engine,
    capabilities: b.capabilities,
    hotkeys: b.hotkeys,
  });
}

export function applyHotkeys(hotkeys: HotkeyStatus[]): void {
  useAppStore.setState({ hotkeys });
}

export function applyTick(tick: EngineTick): void {
  useAppStore.setState({ engine: tick });
}

export function applyStatus(status: EngineStatus): void {
  useAppStore.setState((s) => ({
    lastStatus: status,
    engine: { ...s.engine, state: status.state },
    countdownTotalMs: status.countdownTotalMs ?? s.countdownTotalMs,
  }));
}

export function applySettings(settings: Settings): void {
  useAppStore.setState({ settings });
}

export function applyCapabilities(capabilities: Capabilities): void {
  useAppStore.setState({ capabilities });
}

export function setCommandError(code: string | null): void {
  useAppStore.setState({ commandError: code });
}

/** Replaces one profile in place (keeps the order). */
export function replaceProfile(profile: Profile): void {
  useAppStore.setState((s) => ({ profiles: s.profiles.map((p) => (p.id === profile.id ? profile : p)) }));
}

export function selectActiveProfile(s: AppState): Profile | null {
  const id = s.settings?.activeProfileId;
  return s.profiles.find((p) => p.id === id) ?? null;
}

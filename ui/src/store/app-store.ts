import { create } from "zustand";

import type {
  Bootstrap,
  Capabilities,
  DegradedReason,
  EngineDegraded,
  EngineStatus,
  EngineTick,
  HotkeyStatus,
  Profile,
  Settings,
} from "../ipc";

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

export interface AppState {
  ready: boolean;
  version: string;
  settings: Settings | null;
  profiles: Profile[];
  engine: EngineTick;
  lastStatus: EngineStatus | null;
  countdownTotalMs: number;
  capabilities: Capabilities | null;
  hotkeys: HotkeyStatus[];
  commandError: string | null;
  degraded: DegradedReason | null;
  update: string | null;
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
  degraded: null,
  update: null,
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
    degraded: status.state === "countdown" ? null : s.degraded,
    engine: { ...s.engine, state: status.state },
    countdownTotalMs: status.countdownTotalMs ?? s.countdownTotalMs,
  }));
}

export function applyDegraded(event: EngineDegraded): void {
  useAppStore.setState((s) => ({
    degraded: event.degraded ? event.reason : s.degraded === event.reason ? null : s.degraded,
  }));
}

export function applyUpdate(version: string | null): void {
  useAppStore.setState({ update: version });
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

export function replaceProfile(profile: Profile): void {
  useAppStore.setState((s) => ({ profiles: s.profiles.map((p) => (p.id === profile.id ? profile : p)) }));
}

export function selectActiveProfile(s: AppState): Profile | null {
  const id = s.settings?.activeProfileId;
  return s.profiles.find((p) => p.id === id) ?? null;
}

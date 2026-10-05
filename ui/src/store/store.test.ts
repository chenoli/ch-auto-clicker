import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Bootstrap, Profile, Settings } from "../ipc";

const ipc = vi.hoisted(() => ({
  inTauri: true,
  commands: {
    appBootstrap: vi.fn(),
    profileUpdate: vi.fn(),
    engineStart: vi.fn(),
    engineStop: vi.fn(),
    engineTogglePause: vi.fn(),
    engineAckError: vi.fn(),
  },
  listeners: {} as Record<string, (e: { payload: unknown }) => void>,
}));

vi.mock("../ipc", () => {
  const event = (name: string) => ({
    listen: vi.fn(async (cb: (e: { payload: unknown }) => void) => {
      ipc.listeners[name] = cb;
      return () => delete ipc.listeners[name];
    }),
  });
  return {
    get inTauri() {
      return ipc.inTauri;
    },
    commands: ipc.commands,
    events: {
      engineTick: event("engineTick"),
      engineStatus: event("engineStatus"),
      settingsChanged: event("settingsChanged"),
      profilesChanged: event("profilesChanged"),
      capabilities: event("capabilities"),
      hotkeyStatusChanged: event("hotkeyStatusChanged"),
      hotkeyActionFailed: event("hotkeyActionFailed"),
    },
  };
});

const { IDLE_TICK, applyStatus, selectActiveProfile, useAppStore } = await import("./app-store");
const { applyProfilesChanged, editProfile, hasPendingEdits, resetProfileSync } = await import("./profile-sync");
const { connectStore } = await import("./connect");
const { primaryAction } = await import("./engine-actions");

function profile(id: string, updatedAt = "v1"): Profile {
  return {
    schemaVersion: 1,
    id,
    name: `Perfil ${id}`,
    mode: "click",
    hotkey: null,
    createdAt: "t",
    updatedAt,
    click: { button: "left", kind: "single", intervalMs: 100, variation: { enabled: false, blocksMs: [1, 35, 70] } },
    stopAfter: { enabled: false, seconds: 300 },
    modes: {
      click: { position: "cursor", points: [], layoutFingerprint: null },
      wiggle: { amplitudePct: 0.05, speedHz: 8, clickWhileWiggling: false },
      scan: { area: null, lineSpacingPx: 24, speedPxS: 1200, action: "move", loop: true },
    },
  };
}

function settings(active: string): Settings {
  return { activeProfileId: active, countdownSec: 3 } as Settings;
}

function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

beforeEach(() => {
  resetProfileSync();
  useAppStore.setState({ profiles: [profile("a")], settings: settings("a"), commandError: null, engine: IDLE_TICK });
  applyProfilesChanged({ profiles: [profile("a")], activeProfileId: "a" });
});

afterEach(() => {
  vi.resetAllMocks();
  ipc.inTauri = true;
});

describe("profile sync", () => {
  it("applies edits optimistically and saves against the confirmed version", async () => {
    ipc.commands.profileUpdate.mockImplementation(async (p: Profile) => ({
      status: "ok",
      data: { ...p, updatedAt: "v2" },
    }));
    editProfile("a", (p) => ({ ...p, name: "Novo" }));
    expect(selectActiveProfile(useAppStore.getState())?.name).toBe("Novo");
    await vi.waitFor(() => expect(hasPendingEdits("a")).toBe(false));
    expect(ipc.commands.profileUpdate).toHaveBeenCalledWith(expect.objectContaining({ name: "Novo" }), "v1");
    expect(useAppStore.getState().profiles[0]?.updatedAt).toBe("v2");
  });

  it("serializes rapid edits and bases each save on the previous answer", async () => {
    const first = deferred<unknown>();
    ipc.commands.profileUpdate
      .mockImplementationOnce(() => first.promise)
      .mockImplementationOnce(async (p: Profile) => ({ status: "ok", data: { ...p, updatedAt: "v3" } }));
    editProfile("a", (p) => ({ ...p, name: "1" }));
    editProfile("a", (p) => ({ ...p, name: "2" }));
    editProfile("a", (p) => ({ ...p, name: "3" }));
    expect(ipc.commands.profileUpdate).toHaveBeenCalledTimes(1);
    first.resolve({ status: "ok", data: { ...profile("a", "v2"), name: "1" } });
    await vi.waitFor(() => expect(hasPendingEdits("a")).toBe(false));
    expect(ipc.commands.profileUpdate).toHaveBeenCalledTimes(2);
    expect(ipc.commands.profileUpdate).toHaveBeenLastCalledWith(expect.objectContaining({ name: "3" }), "v2");
    expect(useAppStore.getState().profiles[0]?.name).toBe("3");
  });

  it("rolls back to the confirmed copy when another window saved first", async () => {
    ipc.commands.profileUpdate.mockResolvedValue({
      status: "error",
      error: { code: "STALE_PROFILE", message: "x" },
    });
    editProfile("a", (p) => ({ ...p, name: "Meu" }));
    await vi.waitFor(() => expect(useAppStore.getState().commandError).toBe("STALE_PROFILE"));
    expect(useAppStore.getState().profiles[0]?.name).toBe("Perfil a");
  });

  it("keeps local edits when an event arrives mid-save", async () => {
    const pending = deferred<unknown>();
    ipc.commands.profileUpdate.mockImplementation(() => pending.promise);
    editProfile("a", (p) => ({ ...p, name: "Local" }));
    applyProfilesChanged({ profiles: [{ ...profile("a", "v9"), name: "Remoto" }], activeProfileId: "a" });
    expect(useAppStore.getState().profiles[0]?.name).toBe("Local");
    pending.resolve({ status: "ok", data: { ...profile("a", "v10"), name: "Local" } });
    await vi.waitFor(() => expect(hasPendingEdits("a")).toBe(false));
  });

  it("ignores edits to unknown profiles", () => {
    editProfile("zzz", (p) => p);
    expect(ipc.commands.profileUpdate).not.toHaveBeenCalled();
  });
});

describe("connectStore", () => {
  it("hydrates from the bootstrap and follows events", async () => {
    const boot = {
      version: "0.1.0",
      dpiAwareness: "PerMonitorV2",
      settings: settings("b"),
      profiles: [profile("b")],
      engine: IDLE_TICK,
      capabilities: { minReliableIntervalMs: 1 },
      hotkeys: [],
    } as unknown as Bootstrap;
    ipc.commands.appBootstrap.mockResolvedValue(boot);
    const dispose = await connectStore();
    const s = useAppStore.getState();
    expect(s.ready).toBe(true);
    expect(selectActiveProfile(s)?.id).toBe("b");

    ipc.listeners.engineTick?.({ payload: { ...IDLE_TICK, state: "running", clicks: 5 } });
    expect(useAppStore.getState().engine.clicks).toBe(5);
    ipc.listeners.engineStatus?.({
      payload: { state: "countdown", prevState: "idle", reason: null, countdownTotalMs: 3000, errorCode: null },
    });
    expect(useAppStore.getState().countdownTotalMs).toBe(3000);
    ipc.listeners.settingsChanged?.({ payload: { ...settings("b"), countdownSec: 5 } });
    expect(useAppStore.getState().settings?.countdownSec).toBe(5);
    ipc.listeners.capabilities?.({ payload: { minReliableIntervalMs: 2 } });
    ipc.listeners.hotkeyStatusChanged?.({ payload: [{ ok: false }] });
    expect(useAppStore.getState().hotkeys).toEqual([{ ok: false }]);
    ipc.listeners.hotkeyActionFailed?.({ payload: { code: "NO_AREA" } });
    expect(useAppStore.getState().commandError).toBe("NO_AREA");
    expect(useAppStore.getState().capabilities?.minReliableIntervalMs).toBe(2);
    ipc.listeners.profilesChanged?.({ payload: { profiles: [profile("b", "v5")], activeProfileId: "b" } });
    expect(useAppStore.getState().profiles[0]?.updatedAt).toBe("v5");

    dispose();
    expect(Object.keys(ipc.listeners)).toHaveLength(0);
  });

  it("does nothing outside Tauri", async () => {
    ipc.inTauri = false;
    const dispose = await connectStore();
    dispose();
    expect(ipc.commands.appBootstrap).not.toHaveBeenCalled();
  });
});

describe("primaryAction", () => {
  it("starts, stops, resumes and recovers per state", async () => {
    ipc.commands.engineStart.mockResolvedValue({ status: "ok", data: null });
    await primaryAction("idle");
    expect(ipc.commands.engineStart).toHaveBeenCalledWith(null);
    await primaryAction("running");
    await primaryAction("countdown");
    expect(ipc.commands.engineStop).toHaveBeenCalledTimes(2);
    await primaryAction("paused");
    expect(ipc.commands.engineTogglePause).toHaveBeenCalledOnce();
    await primaryAction("error");
    expect(ipc.commands.engineAckError).toHaveBeenCalledOnce();
    expect(ipc.commands.engineStart).toHaveBeenCalledTimes(2);
    await primaryAction("stopping");
    expect(ipc.commands.engineStop).toHaveBeenCalledTimes(2);
  });

  it("surfaces start errors", async () => {
    ipc.commands.engineStart.mockResolvedValue({ status: "error", error: { code: "NO_AREA", message: "" } });
    await primaryAction("idle");
    expect(useAppStore.getState().commandError).toBe("NO_AREA");
  });

  it("status updates the engine state immediately", () => {
    applyStatus({ state: "paused", prevState: "running", reason: null, countdownTotalMs: null, errorCode: null });
    expect(useAppStore.getState().engine.state).toBe("paused");
  });
});

import type { EngineTick, Profile } from "../../ipc";

export function prototypeProfile(): Profile {
  return {
    schemaVersion: 1,
    id: "proto",
    name: "Coleta rápida",
    mode: "click",
    hotkey: null,
    createdAt: "2026-10-04T00:00:00Z",
    updatedAt: "2026-10-04T00:00:00Z",
    click: { button: "left", kind: "single", intervalMs: 100, variation: { enabled: true, blocksMs: [1, 35, 70] } },
    stopAfter: { enabled: true, seconds: 300 },
    modes: {
      click: {
        position: "points",
        points: [
          { id: "1", x: 412, y: 236, delayMs: 0 },
          { id: "2", x: 1520, y: 400, delayMs: 120 },
          { id: "3", x: 980, y: 760, delayMs: 0 },
          { id: "4", x: 300, y: 820, delayMs: 0 },
        ],
        layoutFingerprint: null,
      },
      wiggle: { amplitudePct: 0.05, speedHz: 8, clickWhileWiggling: false },
      scan: { area: null, lineSpacingPx: 24, speedPxS: 1200, action: "move", loop: true },
    },
  };
}

export function tick(partial: Partial<EngineTick> = {}): EngineTick {
  return {
    state: "idle",
    fires: 0,
    clicks: 0,
    skippedSelf: 0,
    missed: 0,
    elapsedMs: 0,
    countdownRemainingMs: null,
    remainingMs: null,
    ...partial,
  };
}

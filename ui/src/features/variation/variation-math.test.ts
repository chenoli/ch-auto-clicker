import { describe, expect, it } from "vitest";

import { clampBlock, moveBlock, rhythmTicks, variationStats } from "./variation-math";

describe("variationStats", () => {
  it("computes cycle, average and effective CPS", () => {
    expect(variationStats([1, 35, 70], false)).toEqual({ cycleMs: 106, averageMs: 106 / 3, cps: 1000 / (106 / 3) });
    expect(variationStats([100], true).cps).toBe(20);
    expect(variationStats([], false)).toEqual({ cycleMs: 0, averageMs: 0, cps: 0 });
  });
});

describe("rhythmTicks", () => {
  it("marks cycle starts and gaps below the reliable minimum", () => {
    const ticks = rhythmTicks([1, 35, 70], 4);
    expect(ticks[0]).toEqual({ xPct: 0, cycleStart: true, low: true });
    expect(ticks[1]).toMatchObject({ cycleStart: false, low: false });
    expect(ticks[3]?.cycleStart).toBe(true);
    expect(ticks.every((t) => t.xPct <= 100)).toBe(true);
  });

  it("caps very short rhythms and handles empty input", () => {
    expect(rhythmTicks([1], 4).length).toBeLessThanOrEqual(500);
    expect(rhythmTicks([], 4)).toEqual([]);
  });
});

describe("block helpers", () => {
  it("clamps to 1–60000 ms", () => {
    expect(clampBlock(0)).toBe(1);
    expect(clampBlock(70_000)).toBe(60_000);
    expect(clampBlock(12.6)).toBe(13);
    expect(clampBlock(Number.NaN)).toBe(1);
  });

  it("moves blocks within bounds", () => {
    expect(moveBlock([1, 2, 3], 0, 1)).toEqual([[2, 1, 3], 1]);
    expect(moveBlock([1, 2, 3], 0, -1)).toEqual([[1, 2, 3], 0]);
    expect(moveBlock([1, 2, 3], 2, 1)).toEqual([[1, 2, 3], 2]);
  });
});

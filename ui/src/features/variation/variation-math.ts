import { cpsFromInterval } from "../../i18n/format";

export const MAX_BLOCKS = 32;
export const BLOCK_MS_MIN = 1;
export const BLOCK_MS_MAX = 60_000;
export const PREVIEW_MS = 600;

export interface VariationStats {
  cycleMs: number;
  averageMs: number;
  cps: number;
}

export function variationStats(blocks: readonly number[], double: boolean): VariationStats {
  const cycleMs = blocks.reduce((a, b) => a + b, 0);
  const averageMs = blocks.length ? cycleMs / blocks.length : 0;
  return { cycleMs, averageMs, cps: averageMs > 0 ? cpsFromInterval(averageMs, double) : 0 };
}

export interface RhythmTick {
  /** Position in the 600 ms preview, 0–100%. */
  xPct: number;
  /** First click of a cycle: drawn taller and in --fg. */
  cycleStart: boolean;
  /** Gap shorter than the reliable minimum: drawn in --warn. */
  low: boolean;
}

/** Click marks over the first 600 ms of the rhythm (handoff 02.B "Prévia do ritmo"). */
export function rhythmTicks(blocks: readonly number[], minReliableMs: number): RhythmTick[] {
  const ticks: RhythmTick[] = [];
  if (!blocks.length) return ticks;
  let t = 0;
  for (let i = 0; t <= PREVIEW_MS && i < 500; i++) {
    const ms = blocks[i % blocks.length] ?? 1;
    ticks.push({ xPct: (t / PREVIEW_MS) * 100, cycleStart: i % blocks.length === 0, low: ms < minReliableMs });
    t += Math.max(ms, 0.5);
  }
  return ticks;
}

export function clampBlock(ms: number): number {
  if (!Number.isFinite(ms)) return BLOCK_MS_MIN;
  return Math.min(BLOCK_MS_MAX, Math.max(BLOCK_MS_MIN, Math.round(ms)));
}

/** Moves the block at `index` by `delta` positions; returns the new list and selected index. */
export function moveBlock(blocks: readonly number[], index: number, delta: -1 | 1): [number[], number] {
  const target = index + delta;
  if (target < 0 || target >= blocks.length) return [[...blocks], index];
  const next = [...blocks];
  [next[index], next[target]] = [next[target] as number, next[index] as number];
  return [next, target];
}

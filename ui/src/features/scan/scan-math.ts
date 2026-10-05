import type { Area } from "../../ipc";

export const SPACING_MIN = 8;
export const SPACING_MAX = 130;
export const SPACING_STEP = 4;
export const SPEEDS = [600, 1200, 2400] as const;

export const THUMB = { width: 388, height: 130, inset: 4 };

export function scanRows(area: Area, spacing: number): number {
  return Math.floor((Math.max(1, area.h) - 1) / Math.max(1, spacing)) + 1;
}

export function scanLength(area: Area, spacing: number, loop: boolean): number {
  const rows = scanRows(area, spacing);
  const span = Math.max(1, area.w) - 1;
  const open = rows * span + (rows - 1) * spacing;
  if (!loop) return open;
  const endX = rows % 2 === 1 ? span : 0;
  return open + Math.hypot(endX, (rows - 1) * spacing);
}

export function scanCycleSecs(area: Area, spacing: number, speed: number, loop: boolean): number {
  return scanLength(area, spacing, loop) / Math.max(1, speed);
}

const THUMB_ROWS_MAX = 130;

export function thumbPolyline(area: Area, spacing: number): string {
  const total = scanRows(area, spacing);
  const rows = Math.min(total, THUMB_ROWS_MAX);
  const rowAt = (i: number) => (rows === total ? i : Math.round((i * (total - 1)) / (rows - 1)));
  const innerW = THUMB.width - 2 * THUMB.inset;
  const innerH = THUMB.height - 2 * THUMB.inset;
  const sx = (x: number) => THUMB.inset + (x / Math.max(1, area.w - 1)) * innerW;
  const sy = (y: number) => THUMB.inset + (y / Math.max(1, area.h - 1)) * innerH;
  const right = area.w - 1;
  const points: string[] = [];
  for (let row = 0; row < rows; row++) {
    const y = sy(rowAt(row) * spacing);
    const [a, b] = row % 2 === 0 ? [0, right] : [right, 0];
    points.push(`${round(sx(a))},${round(y)}`, `${round(sx(b))},${round(y)}`);
  }
  return points.join(" ");
}

const round = (n: number) => Math.round(n * 10) / 10;

export function clampSpacing(n: number): number {
  return Math.min(SPACING_MAX, Math.max(SPACING_MIN, n));
}

/** A rectangle in overlay CSS pixels; `w`/`h` may be negative while dragging up or left. */
export interface AreaRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type Handle = "nw" | "n" | "ne" | "e" | "se" | "s" | "sw" | "w";

export const HANDLES: readonly Handle[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];

/** Same minimum as Rust (`SCAN_AREA_MIN`), in physical pixels. */
export const AREA_MIN_PX = 16;

interface Size {
  width: number;
  height: number;
}

/** Positive width and height, whatever direction it was dragged in. */
export function normalize(r: AreaRect): AreaRect {
  return {
    x: Math.min(r.x, r.x + r.w),
    y: Math.min(r.y, r.y + r.h),
    w: Math.abs(r.w),
    h: Math.abs(r.h),
  };
}

/** Keeps a normalized rectangle inside the monitor (decision D7: one monitor only). */
export function clampRect(r: AreaRect, bounds: Size): AreaRect {
  const n = normalize(r);
  const x = Math.min(Math.max(0, n.x), bounds.width);
  const y = Math.min(Math.max(0, n.y), bounds.height);
  return { x, y, w: Math.min(n.w, bounds.width - x), h: Math.min(n.h, bounds.height - y) };
}

/** Moves the whole rectangle, stopping at the monitor edges. */
export function moveRect(r: AreaRect, dx: number, dy: number, bounds: Size): AreaRect {
  const n = normalize(r);
  return {
    ...n,
    x: Math.min(Math.max(0, n.x + dx), bounds.width - n.w),
    y: Math.min(Math.max(0, n.y + dy), bounds.height - n.h),
  };
}

/** Drags one of the 8 handles; edges may cross (the result is normalized), never the monitor. */
export function resizeRect(r: AreaRect, handle: Handle, dx: number, dy: number, bounds: Size): AreaRect {
  const n = normalize(r);
  let left = n.x;
  let top = n.y;
  let right = n.x + n.w;
  let bottom = n.y + n.h;
  if (handle.includes("w")) left += dx;
  if (handle.includes("e")) right += dx;
  if (handle.includes("n")) top += dy;
  if (handle.includes("s")) bottom += dy;
  return clampRect({ x: left, y: top, w: right - left, h: bottom - top }, bounds);
}

/** Size in physical pixels, as the HUD shows it ("776 × 260"). */
export function physicalSize(r: AreaRect, scale: number): { w: number; h: number } {
  const n = normalize(r);
  return { w: Math.round(n.w * scale), h: Math.round(n.h * scale) };
}

export function isUsable(r: AreaRect | null, scale: number): r is AreaRect {
  if (!r) return false;
  const size = physicalSize(r, scale);
  return size.w >= AREA_MIN_PX && size.h >= AREA_MIN_PX;
}

const HUD_GAP = 16;
const HUD_HEIGHT = 40;

/** The HUD sits under the rectangle, or above it when there is no room below. */
export function hudTop(r: AreaRect, viewportHeight: number): number {
  const n = normalize(r);
  const below = n.y + n.h + HUD_GAP;
  return below + HUD_HEIGHT <= viewportHeight ? below : Math.max(0, n.y - HUD_GAP - HUD_HEIGHT);
}

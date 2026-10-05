import type { OverlayMonitor } from "../../ipc";

export interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

export const MINIMAP: Box = { x: 0, y: 0, width: 388, height: 170 };

/** Bounding box of every monitor (the virtual desktop). */
export function desktopBounds(monitors: readonly OverlayMonitor[]): Box {
  if (!monitors.length) return { x: 0, y: 0, width: 1920, height: 1080 };
  const left = Math.min(...monitors.map((m) => m.x));
  const top = Math.min(...monitors.map((m) => m.y));
  const right = Math.max(...monitors.map((m) => m.x + m.width));
  const bottom = Math.max(...monitors.map((m) => m.y + m.height));
  return { x: left, y: top, width: right - left, height: bottom - top };
}

/**
 * Maps a desktop pixel into the minimap. Like the handoff, the desktop is stretched to fill the box
 * (1920 × 1080 → 388 × 170), so positions stay comparable at a glance.
 */
export function toMinimap(desktop: Box, x: number, y: number, box: Box = MINIMAP): { x: number; y: number } {
  return {
    x: box.x + ((x - desktop.x) / desktop.width) * box.width,
    y: box.y + ((y - desktop.y) / desktop.height) * box.height,
  };
}

export interface Arrow {
  x: number;
  y: number;
  angleDeg: number;
}

/** Direction arrows at the midpoint of each segment of the click order. */
export function segmentArrows(path: readonly { x: number; y: number }[]): Arrow[] {
  const arrows: Arrow[] = [];
  for (let i = 1; i < path.length; i++) {
    const a = path[i - 1];
    const b = path[i];
    if (!a || !b) continue;
    arrows.push({
      x: (a.x + b.x) / 2,
      y: (a.y + b.y) / 2,
      angleDeg: (Math.atan2(b.y - a.y, b.x - a.x) * 180) / Math.PI,
    });
  }
  return arrows;
}

/** Monitor DPI scale; specta types f64 as nullable, Rust always sends a number. */
export const scaleOf = (m: OverlayMonitor): number => m.scale ?? 1;

/** Desktop pixel → CSS position inside the overlay of `monitor`. */
export function toOverlay(monitor: OverlayMonitor, x: number, y: number): { x: number; y: number } {
  const scale = scaleOf(monitor);
  return { x: (x - monitor.x) / scale, y: (y - monitor.y) / scale };
}

/** CSS position inside the overlay → desktop pixel (rounded like Rust does). */
export function toDesktop(monitor: OverlayMonitor, cssX: number, cssY: number): { x: number; y: number } {
  const scale = scaleOf(monitor);
  return { x: monitor.x + Math.round(cssX * scale), y: monitor.y + Math.round(cssY * scale) };
}

export const LOUPE_OFFSET = 24;
const LOUPE_BLOCK = { width: 160, height: 210 };

/** Loupe sits 24 px below-right of the cursor, flipping to the other side near the edges. */
export function loupePosition(cursor: { x: number; y: number }, viewport: { width: number; height: number }) {
  const right = cursor.x + LOUPE_OFFSET + LOUPE_BLOCK.width <= viewport.width;
  const below = cursor.y + LOUPE_OFFSET + LOUPE_BLOCK.height <= viewport.height;
  return {
    left: right ? cursor.x + LOUPE_OFFSET : cursor.x - LOUPE_OFFSET - LOUPE_BLOCK.width,
    top: below ? cursor.y + LOUPE_OFFSET : cursor.y - LOUPE_OFFSET - LOUPE_BLOCK.height,
  };
}

/** `#5C8CEB` for an RGB triple. */
export function hexColor(r: number, g: number, b: number): string {
  return `#${[r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")}`.toUpperCase();
}

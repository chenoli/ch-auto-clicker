import type { OverlayMonitor } from "../../ipc";

export interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

export const MINIMAP: Box = { x: 0, y: 0, width: 388, height: 170 };

export function desktopBounds(monitors: readonly OverlayMonitor[]): Box {
  if (!monitors.length) return { x: 0, y: 0, width: 1920, height: 1080 };
  const left = Math.min(...monitors.map((m) => m.x));
  const top = Math.min(...monitors.map((m) => m.y));
  const right = Math.max(...monitors.map((m) => m.x + m.width));
  const bottom = Math.max(...monitors.map((m) => m.y + m.height));
  return { x: left, y: top, width: right - left, height: bottom - top };
}

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

export const scaleOf = (m: OverlayMonitor): number => m.scale ?? 1;

export function toOverlay(monitor: OverlayMonitor, x: number, y: number): { x: number; y: number } {
  const scale = scaleOf(monitor);
  return { x: (x - monitor.x) / scale, y: (y - monitor.y) / scale };
}

export function toDesktop(monitor: OverlayMonitor, cssX: number, cssY: number): { x: number; y: number } {
  const scale = scaleOf(monitor);
  return { x: monitor.x + Math.round(cssX * scale), y: monitor.y + Math.round(cssY * scale) };
}

export const LOUPE_OFFSET = 24;
const LOUPE_BLOCK = { width: 160, height: 210 };

export function loupePosition(cursor: { x: number; y: number }, viewport: { width: number; height: number }) {
  const right = cursor.x + LOUPE_OFFSET + LOUPE_BLOCK.width <= viewport.width;
  const below = cursor.y + LOUPE_OFFSET + LOUPE_BLOCK.height <= viewport.height;
  return {
    left: right ? cursor.x + LOUPE_OFFSET : cursor.x - LOUPE_OFFSET - LOUPE_BLOCK.width,
    top: below ? cursor.y + LOUPE_OFFSET : cursor.y - LOUPE_OFFSET - LOUPE_BLOCK.height,
  };
}

export function hexColor(r: number, g: number, b: number): string {
  return `#${[r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")}`.toUpperCase();
}

import { describe, expect, it } from "vitest";

import { desktopBounds, hexColor, loupePosition, segmentArrows, toDesktop, toMinimap, toOverlay } from "./geometry";

import type { OverlayMonitor } from "../../ipc";

const primary: OverlayMonitor = { index: 0, x: 0, y: 0, width: 1920, height: 1080, scale: 1 };
const left: OverlayMonitor = { index: 1, x: -1680, y: 0, width: 1680, height: 1050, scale: 1.25 };

describe("desktop and minimap", () => {
  it("bounds every monitor, including negative origins", () => {
    expect(desktopBounds([primary, left])).toEqual({ x: -1680, y: 0, width: 3600, height: 1080 });
    expect(desktopBounds([])).toEqual({ x: 0, y: 0, width: 1920, height: 1080 });
  });

  it("stretches the desktop into the 388 × 170 box like the handoff", () => {
    const desk = desktopBounds([primary]);
    expect(toMinimap(desk, 0, 0)).toEqual({ x: 0, y: 0 });
    expect(toMinimap(desk, 1920, 1080)).toEqual({ x: 388, y: 170 });
    expect(toMinimap(desk, 412, 236).x).toBeCloseTo((412 / 1920) * 388);
  });

  it("puts an arrow at the middle of every segment, pointing forward", () => {
    const arrows = segmentArrows([
      { x: 0, y: 0 },
      { x: 10, y: 0 },
      { x: 10, y: 10 },
    ]);
    expect(arrows).toEqual([
      { x: 5, y: 0, angleDeg: 0 },
      { x: 10, y: 5, angleDeg: 90 },
    ]);
    expect(segmentArrows([{ x: 1, y: 1 }])).toEqual([]);
  });
});

describe("overlay coordinates", () => {
  it("round-trips between desktop pixels and overlay CSS pixels", () => {
    expect(toOverlay(left, -840, 610)).toEqual({ x: 672, y: 488 });
    expect(toDesktop(left, 672, 488)).toEqual({ x: -840, y: 610 });
    expect(toDesktop(primary, 10.4, 0.6)).toEqual({ x: 10, y: 1 });
  });

  it("flips the loupe near the right and bottom edges", () => {
    const viewport = { width: 1920, height: 1080 };
    expect(loupePosition({ x: 100, y: 100 }, viewport)).toEqual({ left: 124, top: 124 });
    expect(loupePosition({ x: 1900, y: 1000 }, viewport)).toEqual({ left: 1716, top: 766 });
  });

  it("formats colors as upper-case hex", () => {
    expect(hexColor(92, 140, 235)).toBe("#5C8CEB");
    expect(hexColor(0, 0, 0)).toBe("#000000");
  });
});

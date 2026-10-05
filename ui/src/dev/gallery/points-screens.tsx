import { useState } from "react";

import { AreaView } from "../../features/overlay/area-view";
import { OverlayView } from "../../features/overlay/overlay-view";
import { PointsScreen } from "../../features/points/points-screen";

import { WindowFrame } from "./window-frame";

import type { OverlayState, ScreenPoint } from "../../ipc";
import type { ReactElement } from "react";

const noop = () => {};

/** The four points of the handoff (Tela 02.A). */
const HANDOFF_POINTS: ScreenPoint[] = [
  { id: "1", x: 412, y: 236, delayMs: 0 },
  { id: "2", x: 1284, y: 612, delayMs: 120 },
  { id: "3", x: 980, y: 884, delayMs: 0 },
  { id: "4", x: 520, y: 700, delayMs: 250 },
];

/** Tela 02.A, editable in place so drag and drop can be tried in the gallery. */
export function PointsScreenDemo(): ReactElement {
  const [points, setPoints] = useState(HANDOFF_POINTS);
  return (
    <WindowFrame width={420} height={640}>
      {() => (
        <PointsScreen
          points={points}
          monitors={[{ index: 0, x: 0, y: 0, width: 1920, height: 1080, scale: 1 }]}
          layoutChanged={false}
          captureShortcut="Ctrl Shift P"
          onChange={setPoints}
          onCapture={noop}
          onAddManual={noop}
          onBack={noop}
          onMinimize={noop}
          onClose={noop}
        />
      )}
    </WindowFrame>
  );
}

// The handoff artboard is 1920 × 1080 at 50%, so the demo monitor has scale 2.
const OVERLAY_STATE: OverlayState = {
  monitor: { index: 0, x: 0, y: 0, width: 1920, height: 1080, scale: 2 },
  points: [
    { number: 1, x: 412, y: 236 },
    { number: 2, x: 1284, y: 612 },
    { number: 3, x: 980, y: 884 },
    { number: 4, x: 520, y: 700 },
  ],
  nextNumber: 5,
  mode: "points",
  area: null,
};

/** A stand-in capture: dark stripes with a blue bar under the crosshair, like the handoff loupe. */
function fakeCapture(): HTMLCanvasElement | null {
  if (typeof document === "undefined") return null;
  const canvas = document.createElement("canvas");
  canvas.width = 1920;
  canvas.height = 1080;
  const ctx = canvas.getContext("2d");
  if (!ctx) return null;
  ctx.fillStyle = "#2A2C30";
  ctx.fillRect(0, 0, 1920, 1080);
  ctx.fillStyle = "#26282C";
  for (let x = -1080; x < 1920; x += 40) {
    ctx.beginPath();
    ctx.moveTo(x, 1080);
    ctx.lineTo(x + 20, 1080);
    ctx.lineTo(x + 1100, 0);
    ctx.lineTo(x + 1080, 0);
    ctx.fill();
  }
  ctx.fillStyle = "#5C8CEB";
  ctx.fillRect(1480, 400, 80, 2);
  ctx.fillStyle = "#E6E6E6";
  ctx.fillRect(1520, 402, 40, 2);
  return canvas;
}

/** Tela 03.B at the handoff's 50% artboard size. */
export function OverlayScreenDemo(): ReactElement {
  const [capture] = useState(fakeCapture);
  return (
    // `transform` makes the fixed overlay root position itself inside this artboard.
    <div className="relative overflow-hidden" style={{ width: 960, height: 540, transform: "translateZ(0)" }}>
      <OverlayView
        state={OVERLAY_STATE}
        image={capture}
        imageUrl={null}
        cursor={{ x: 1520, y: 400 }}
        viewport={{ width: 960, height: 540 }}
        fallbackColor="#5C8CEB"
      />
    </div>
  );
}

/** Tela 03.A at the handoff's 50% artboard size: 776 × 260 at X 480 · Y 300. */
export function AreaOverlayScreenDemo(): ReactElement {
  return (
    <div className="relative overflow-hidden" style={{ width: 960, height: 540, transform: "translateZ(0)" }}>
      <AreaView
        state={{ ...OVERLAY_STATE, mode: "area", points: [] }}
        imageUrl={null}
        rect={{ x: 240, y: 150, w: 388, h: 130 }}
        viewport={{ width: 960, height: 540 }}
        onMode={noop}
        onFinish={noop}
      />
    </div>
  );
}

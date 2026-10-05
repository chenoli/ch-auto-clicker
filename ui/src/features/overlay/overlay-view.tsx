import { useEffect, useRef, useState } from "react";

import { useTranslation } from "react-i18next";

import { hexColor, loupePosition, scaleOf, toOverlay } from "../points/geometry";
import { OverlayBar } from "./overlay-bar";

import type { OverlayMode, OverlayState } from "../../ipc";
import type { ReactElement } from "react";

export const LOUPE_PIXELS = 11;
export const LOUPE_CELL = 12;
const LOUPE_SIZE = LOUPE_PIXELS * LOUPE_CELL;

export interface OverlayViewProps {
  state: OverlayState;
  onFinish?: () => void;
  onMode?: (mode: OverlayMode) => void;
  image: CanvasImageSource | null;
  imageUrl: string | null;
  cursor: { x: number; y: number } | null;
  viewport: { width: number; height: number };
  fallbackColor?: string;
}

function useLoupe(image: CanvasImageSource | null, cursor: { x: number; y: number } | null) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [color, setColor] = useState<string | null>(null);
  useEffect(() => {
    const ctx = canvasRef.current?.getContext("2d", { willReadFrequently: true });
    if (!ctx || !image || !cursor) return;
    const half = Math.floor(LOUPE_PIXELS / 2);
    ctx.imageSmoothingEnabled = false;
    ctx.fillStyle = "#000";
    ctx.fillRect(0, 0, LOUPE_SIZE, LOUPE_SIZE);
    ctx.drawImage(image, cursor.x - half, cursor.y - half, LOUPE_PIXELS, LOUPE_PIXELS, 0, 0, LOUPE_SIZE, LOUPE_SIZE);
    try {
      const [r = 0, g = 0, b = 0] = ctx.getImageData(half * LOUPE_CELL + 1, half * LOUPE_CELL + 1, 1, 1).data;
      setColor(hexColor(r, g, b));
    } catch {
      setColor(null);
    }
  }, [image, cursor]);
  return { canvasRef, color };
}

export function OverlayView({
  state,
  image,
  imageUrl,
  cursor,
  viewport,
  fallbackColor,
  onFinish,
  onMode,
}: OverlayViewProps): ReactElement {
  const { t } = useTranslation();
  const { canvasRef, color } = useLoupe(image, cursor);
  const { monitor } = state;
  const scale = scaleOf(monitor);
  const css = cursor ? { x: cursor.x / scale, y: cursor.y / scale } : null;
  const loupe = css ? loupePosition(css, viewport) : null;
  const shownColor = color ?? fallbackColor ?? null;
  const points = state.points
    .map((p) => ({ number: p.number, ...toOverlay(monitor, p.x, p.y) }))
    .filter((p) => p.x >= 0 && p.y >= 0 && p.x <= viewport.width && p.y <= viewport.height);

  return (
    <div
      className={`overlay-root ${imageUrl ? "" : "overlay-root--placeholder"}`}
      style={imageUrl ? { backgroundImage: `url("${imageUrl}")` } : undefined}
    >
      <div className="overlay-dim" />
      {points.map((p) => (
        <span key={p.number} className="point-marker point-marker--large" style={{ left: p.x, top: p.y }}>
          {p.number}
        </span>
      ))}
      {css && cursor && loupe && (
        <>
          <div className="crosshair-v" style={{ left: css.x }} />
          <div className="crosshair-h" style={{ top: css.y }} />
          <span className="crosshair-label" style={{ left: css.x + 6, top: 6 }}>
            x {monitor.x + cursor.x}
          </span>
          <span className="crosshair-label" style={{ left: 6, top: css.y - 16 }}>
            y {monitor.y + cursor.y}
          </span>
          <div className="cursor-ring" style={{ left: css.x, top: css.y }} />
          <div className="absolute flex flex-col items-start gap-2" style={{ left: loupe.left, top: loupe.top }}>
            <div className="loupe">
              <canvas ref={canvasRef} width={LOUPE_SIZE} height={LOUPE_SIZE} />
              <span className="loupe__center" />
            </div>
            <div className="floating-card">
              <span className="font-display text-15 font-extrabold text-fg tabular-nums">
                {monitor.x + cursor.x}, {monitor.y + cursor.y}
              </span>
              {shownColor && (
                <span className="flex items-center gap-1.5 text-11 font-semibold text-fg-2">
                  <span className="size-2.5 rounded-[2px]" style={{ background: shownColor }} />
                  {shownColor}
                </span>
              )}
            </div>
          </div>
        </>
      )}
      <OverlayBar mode="points" onMode={onMode} onFinish={onFinish}>
        <span>
          {t("overlay.addPoint")}{" "}
          <span className="font-bold text-accent">{String(state.nextNumber).padStart(2, "0")}</span>
        </span>
        <span className="overlay-bar__sep" />
        <span>{t("overlay.arrows")}</span>
        <span className="overlay-bar__sep" />
        <span>{t("overlay.undo")}</span>
        <span className="overlay-bar__sep" />
      </OverlayBar>
    </div>
  );
}

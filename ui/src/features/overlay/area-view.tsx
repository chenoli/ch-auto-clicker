import { useTranslation } from "react-i18next";

import { scaleOf } from "../points/geometry";
import { HANDLES, hudTop, isUsable, normalize, physicalSize } from "./area-geometry";
import { OverlayBar } from "./overlay-bar";

import type { OverlayMode, OverlayState } from "../../ipc";
import type { AreaRect, Handle } from "./area-geometry";
import type { PointerEvent as ReactPointerEvent, ReactElement } from "react";

export interface AreaViewProps {
  state: OverlayState;
  imageUrl: string | null;
  rect: AreaRect | null;
  viewport: { width: number; height: number };
  onHandleDown?: (handle: Handle, e: ReactPointerEvent) => void;
  onBodyDown?: (e: ReactPointerEvent) => void;
  onConfirm?: () => void;
  onMode?: (mode: OverlayMode) => void;
  onFinish?: () => void;
}

export function AreaView({
  state,
  imageUrl,
  rect,
  viewport,
  onHandleDown,
  onBodyDown,
  onConfirm,
  onMode,
  onFinish,
}: AreaViewProps): ReactElement {
  const { t } = useTranslation();
  const { monitor } = state;
  const scale = scaleOf(monitor);
  const r = rect ? normalize(rect) : null;
  const size = r ? physicalSize(r, scale) : null;
  const origin = r ? { x: monitor.x + Math.round(r.x * scale), y: monitor.y + Math.round(r.y * scale) } : null;
  const usable = isUsable(r, scale);

  return (
    <div
      className={`overlay-root overlay-root--area ${imageUrl ? "" : "overlay-root--placeholder"}`}
      style={imageUrl ? { backgroundImage: `url("${imageUrl}")` } : undefined}
    >
      {!r && <div className="overlay-dim overlay-dim--area" />}
      {r && origin && size && (
        <>
          <div
            className="area-rect"
            style={{ left: r.x, top: r.y, width: r.w, height: r.h }}
            onPointerDown={(e) => {
              e.stopPropagation();
              onBodyDown?.(e);
            }}
          >
            <div className="area-rect__grid">
              {Array.from({ length: 9 }, (_, i) => (
                <span key={i} />
              ))}
            </div>
            {HANDLES.map((h) => (
              <span
                key={h}
                className="area-handle"
                data-handle={h}
                onPointerDown={(e) => {
                  e.stopPropagation();
                  onHandleDown?.(h, e);
                }}
              />
            ))}
            <span className="area-rect__origin">
              {origin.x}, {origin.y}
            </span>
          </div>
          <div
            className="area-hud"
            style={{ left: r.x, top: hudTop(r, viewport.height) }}
            onPointerDown={(e) => e.stopPropagation()}
          >
            <span className="font-display text-17 font-extrabold tracking-logo text-fg tabular-nums">
              {size.w} × {size.h}
            </span>
            <span className="overlay-bar__sep" />
            <span className="text-11 font-semibold text-fg-2 tabular-nums">
              X {origin.x} · Y {origin.y}
            </span>
            <button type="button" className="area-hud__confirm" disabled={!usable} onClick={onConfirm}>
              {t("overlay.confirm")}
            </button>
          </div>
        </>
      )}
      <OverlayBar mode="area" onMode={onMode} onFinish={onFinish}>
        <span>{t("overlay.dragToSelect")}</span>
      </OverlayBar>
    </div>
  );
}

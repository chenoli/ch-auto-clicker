import { useEffect, useRef, useState } from "react";

import { getCurrentWindow } from "@tauri-apps/api/window";

import { commands, events } from "../../ipc";

import { scaleOf, toOverlay } from "../points/geometry";
import { clampRect, isUsable, moveRect, normalize, resizeRect } from "./area-geometry";
import { AreaView } from "./area-view";
import { captureUrl, clampToMonitor, overlayKeyAction } from "./overlay-keys";
import { OverlayView } from "./overlay-view";

import type { OverlayMode, OverlayState } from "../../ipc";
import type { AreaRect, Handle } from "./area-geometry";
import type { PointerEvent as ReactPointerEvent, ReactElement } from "react";

// overlay.html?m=<monitor>: every overlay window is fixed to one monitor for its whole life.
const INDEX = Number(new URLSearchParams(window.location.search).get("m") ?? 0);
const CAPTURE_URL = captureUrl(INDEX);

/** A press must travel this far before it starts a new rectangle, so a stray click keeps the old one. */
const DRAW_THRESHOLD = 4;

type Drag =
  | { kind: "draw"; x: number; y: number }
  | { kind: "move"; x: number; y: number; from: AreaRect }
  | { kind: "resize"; handle: Handle; x: number; y: number; from: AreaRect };

function refresh(setState: (s: OverlayState) => void) {
  void commands.overlayState(INDEX).then((r) => {
    if (r.status === "ok") setState(r.data);
  });
}

/** The profile's saved area, when it lies on this monitor, in this overlay's CSS pixels. */
export function savedRect(state: OverlayState): AreaRect | null {
  const { area, monitor } = state;
  if (!area) return null;
  const inside =
    area.x >= monitor.x &&
    area.y >= monitor.y &&
    area.x + area.w <= monitor.x + monitor.width &&
    area.y + area.h <= monitor.y + monitor.height;
  if (!inside) return null;
  const a = toOverlay(monitor, area.x, area.y);
  const scale = scaleOf(monitor);
  return { x: a.x, y: a.y, w: area.w / scale, h: area.h / scale };
}

/** One capture overlay window, connected to the Rust session (Ponto or Área). */
export function OverlayWindow(): ReactElement | null {
  const [state, setState] = useState<OverlayState | null>(null);
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [cursor, setCursor] = useState<{ x: number; y: number } | null>(null);
  const [drawn, setDrawn] = useState<AreaRect | null>(null);
  const [viewport, setViewport] = useState({ width: window.innerWidth, height: window.innerHeight });
  const drag = useRef<Drag | null>(null);

  useEffect(() => {
    refresh(setState);
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => setImage(img);
    img.src = CAPTURE_URL;
    const onResize = () => setViewport({ width: window.innerWidth, height: window.innerHeight });
    window.addEventListener("resize", onResize);
    // Points added in another monitor's overlay renumber this one too; the mode is shared.
    const unlisten = [
      events.profilesChanged.listen(() => refresh(setState)),
      events.overlayChanged.listen(() => refresh(setState)),
    ];
    return () => {
      window.removeEventListener("resize", onResize);
      for (const u of unlisten) void u.then((f) => f());
    };
  }, []);

  const rect = drawn ?? (state ? savedRect(state) : null);
  const scale = state ? scaleOf(state.monitor) : 1;

  const addAt = (pos: { x: number; y: number }) => {
    void commands.overlayAddPoint(INDEX, pos.x / scale, pos.y / scale).then((r) => {
      if (r.status === "ok") setState(r.data);
    });
  };
  const confirm = () => {
    if (!isUsable(rect, scale)) return;
    const r = normalize(rect);
    void commands.overlayConfirmArea(INDEX, r.x, r.y, r.w, r.h);
  };
  const finish = () => void commands.overlayFinish();
  const setMode = (mode: OverlayMode) => void commands.overlaySetMode(mode);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (state?.mode === "area") {
        if (e.key === "Enter") confirm();
        else if (e.key === "Escape") finish();
        else return;
        e.preventDefault();
        return;
      }
      const action = overlayKeyAction(e);
      if (action.kind === "none") return;
      e.preventDefault();
      switch (action.kind) {
        case "nudge":
          if (cursor && state) {
            setCursor(clampToMonitor({ x: cursor.x + action.dx, y: cursor.y + action.dy }, state.monitor));
          }
          return;
        case "add":
          if (cursor) addAt(cursor);
          return;
        case "undo":
          void commands.overlayUndo();
          return;
        case "finish":
          finish();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  if (!state) return null;
  const local = (e: ReactPointerEvent) => ({
    x: Math.min(Math.max(0, e.clientX), viewport.width),
    y: Math.min(Math.max(0, e.clientY), viewport.height),
  });
  const physical = (e: ReactPointerEvent) =>
    clampToMonitor({ x: Math.round(e.clientX * scale), y: Math.round(e.clientY * scale) }, state.monitor);
  const capture = (e: ReactPointerEvent) => (e.target as Element).setPointerCapture?.(e.pointerId);

  if (state.mode === "area") {
    return (
      <div
        onPointerEnter={() => void getCurrentWindow().setFocus()}
        onPointerDown={(e) => {
          if (e.button !== 0) return;
          const p = local(e);
          capture(e);
          drag.current = { kind: "draw", ...p };
        }}
        onPointerMove={(e) => {
          const d = drag.current;
          if (!d) return;
          const p = local(e);
          if (d.kind === "draw") {
            if (Math.hypot(p.x - d.x, p.y - d.y) < DRAW_THRESHOLD) return;
            setDrawn(clampRect({ x: d.x, y: d.y, w: p.x - d.x, h: p.y - d.y }, viewport));
          } else if (d.kind === "move") setDrawn(moveRect(d.from, p.x - d.x, p.y - d.y, viewport));
          else setDrawn(resizeRect(d.from, d.handle, p.x - d.x, p.y - d.y, viewport));
        }}
        onPointerUp={() => {
          drag.current = null;
        }}
        onContextMenu={(e) => e.preventDefault()}
      >
        <AreaView
          state={state}
          imageUrl={CAPTURE_URL}
          rect={rect}
          viewport={viewport}
          onBodyDown={(e) => {
            if (!rect) return;
            capture(e);
            drag.current = { kind: "move", ...local(e), from: rect };
          }}
          onHandleDown={(handle, e) => {
            if (!rect) return;
            capture(e);
            drag.current = { kind: "resize", handle, ...local(e), from: rect };
          }}
          onConfirm={confirm}
          onMode={setMode}
          onFinish={finish}
        />
      </div>
    );
  }

  return (
    <div
      onPointerMove={(e) => setCursor(physical(e))}
      onPointerEnter={() => void getCurrentWindow().setFocus()}
      onPointerLeave={() => setCursor(null)}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        const pos = physical(e);
        setCursor(pos);
        addAt(pos);
      }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <OverlayView
        state={state}
        image={image}
        imageUrl={CAPTURE_URL}
        cursor={cursor}
        viewport={viewport}
        onFinish={finish}
        onMode={setMode}
      />
    </div>
  );
}

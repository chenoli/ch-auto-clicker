import { useEffect, useState } from "react";

import { getCurrentWindow } from "@tauri-apps/api/window";

import { commands, events } from "../../ipc";

import { scaleOf } from "../points/geometry";
import { captureUrl, clampToMonitor, overlayKeyAction } from "./overlay-keys";
import { OverlayView } from "./overlay-view";

import type { OverlayState } from "../../ipc";
import type { ReactElement } from "react";

// overlay.html?m=<monitor>: every overlay window is fixed to one monitor for its whole life.
const INDEX = Number(new URLSearchParams(window.location.search).get("m") ?? 0);
const CAPTURE_URL = captureUrl(INDEX);

function refresh(setState: (s: OverlayState) => void) {
  void commands.overlayState(INDEX).then((r) => {
    if (r.status === "ok") setState(r.data);
  });
}

/** One capture overlay window, connected to the Rust session. */
export function OverlayWindow(): ReactElement | null {
  const [state, setState] = useState<OverlayState | null>(null);
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [cursor, setCursor] = useState<{ x: number; y: number } | null>(null);
  const [viewport, setViewport] = useState({ width: window.innerWidth, height: window.innerHeight });

  useEffect(() => {
    refresh(setState);
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => setImage(img);
    img.src = CAPTURE_URL;
    const onResize = () => setViewport({ width: window.innerWidth, height: window.innerHeight });
    window.addEventListener("resize", onResize);
    // Points added in another monitor's overlay renumber this one too.
    const unlisten = events.profilesChanged.listen(() => refresh(setState));
    return () => {
      window.removeEventListener("resize", onResize);
      void unlisten.then((f) => f());
    };
  }, []);

  const addAt = (pos: { x: number; y: number }) => {
    if (!state) return;
    const scale = scaleOf(state.monitor);
    void commands.overlayAddPoint(INDEX, pos.x / scale, pos.y / scale).then((r) => {
      if (r.status === "ok") setState(r.data);
    });
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const action = overlayKeyAction(e);
      if (action.kind === "none") return;
      e.preventDefault();
      const current = cursor;
      switch (action.kind) {
        case "nudge":
          if (current && state) {
            setCursor(clampToMonitor({ x: current.x + action.dx, y: current.y + action.dy }, state.monitor));
          }
          return;
        case "add":
          if (current) addAt(current);
          return;
        case "undo":
          void commands.overlayUndo();
          return;
        case "finish":
          void commands.overlayFinish();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  if (!state) return null;
  const scale = scaleOf(state.monitor);
  const physical = (e: React.PointerEvent) =>
    clampToMonitor({ x: Math.round(e.clientX * scale), y: Math.round(e.clientY * scale) }, state.monitor);

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
        onFinish={() => void commands.overlayFinish()}
      />
    </div>
  );
}

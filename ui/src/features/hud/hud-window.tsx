import { useEffect, useRef, useState } from "react";

import { getCurrentWindow } from "@tauri-apps/api/window";

import { commands, events, inTauri } from "../../ipc";

import { selectActiveProfile, useAppStore } from "../../store/app-store";
import { connectStoreEffect } from "../../store/connect";

import { hotkeyLabel } from "../hotkeys/format-hotkey";
import { HudView } from "./hud-view";

import type { ReactElement } from "react";

const PAD = 12;

export function HudWindow(): ReactElement | null {
  const engine = useAppStore((s) => s.engine);
  const settings = useAppStore((s) => s.settings);
  const profile = useAppStore(selectActiveProfile);
  const [hover, setHover] = useState({ hover: false, draggable: false });
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const disconnect = connectStoreEffect();
    const unlisten = inTauri ? events.hudHover.listen((e) => setHover(e.payload)) : null;
    return () => {
      disconnect();
      void unlisten?.then((f) => f());
    };
  }, []);

  const ready = settings !== null;
  useEffect(() => {
    const root = rootRef.current;
    if (!ready || !root || !inTauri) return;
    const observer = new ResizeObserver(() => {
      const box = root.getBoundingClientRect();
      void commands.hudResize(Math.ceil(box.width) + PAD * 2, Math.ceil(box.height) + PAD * 2);
    });
    observer.observe(root);
    return () => observer.disconnect();
  }, [ready]);

  if (!settings) return null;
  return (
    <div className="hud-root" style={{ padding: PAD }}>
      <div ref={rootRef} className="inline-block">
        <HudView
          state={engine.state}
          locale={settings.locale}
          profileName={profile?.name ?? ""}
          clicks={engine.clicks}
          countdownRemainingMs={engine.countdownRemainingMs}
          remainingMs={engine.remainingMs}
          blockedByAdmin={engine.blockedByAdmin}
          startStop={hotkeyLabel(settings.hotkeys.startStop)}
          pauseResume={hotkeyLabel(settings.hotkeys.pauseResume)}
          hover={hover.hover}
          draggable={hover.draggable}
          onDragStart={() => void getCurrentWindow().startDragging()}
        />
      </div>
    </div>
  );
}

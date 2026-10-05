import { useEffect, useRef, useState } from "react";

import { getCurrentWindow } from "@tauri-apps/api/window";

import { commands, events, inTauri } from "../../ipc";

import { selectActiveProfile, useAppStore } from "../../store/app-store";
import { connectStore } from "../../store/connect";

import { hotkeyLabel } from "../hotkeys/format-hotkey";
import { HudView } from "./hud-view";

import type { ReactElement } from "react";

/** Extra room around the pill for its shadow and the hint line. */
const PAD = 12;

/** The HUD window: engine ticks from the store, hover/drag state from Rust (T11.11–T11.12). */
export function HudWindow(): ReactElement | null {
  const engine = useAppStore((s) => s.engine);
  const settings = useAppStore((s) => s.settings);
  const profile = useAppStore(selectActiveProfile);
  const [hover, setHover] = useState({ hover: false, draggable: false });
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    void connectStore().then((d) => (dispose = d));
    const unlisten = inTauri ? events.hudHover.listen((e) => setHover(e.payload)) : null;
    return () => {
      dispose?.();
      void unlisten?.then((f) => f());
    };
  }, []);

  // The window hugs the pill so its transparent area never covers anything. Re-run once the
  // settings arrive: before that the component renders nothing and there is no element to watch.
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
  const hints = settings.hudHintsShown ?? 0;
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
          startStop={hotkeyLabel(settings.hotkeys.startStop)}
          pauseResume={hotkeyLabel(settings.hotkeys.pauseResume)}
          hover={hover.hover}
          draggable={hover.draggable}
          showHint={hints > 0 && hints <= 3}
          onDragStart={() => void getCurrentWindow().startDragging()}
        />
      </div>
    </div>
  );
}

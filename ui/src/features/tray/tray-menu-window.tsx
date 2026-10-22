import { useEffect, useRef, useState } from "react";

import { commands, events, inTauri } from "../../ipc";

import { selectActiveProfile, useAppStore } from "../../store/app-store";
import { connectStoreEffect } from "../../store/connect";

import { hotkeyLabel } from "../hotkeys/format-hotkey";
import { TrayMenuView } from "./tray-menu-view";

import type { ReactElement } from "react";

const PAD = 16;
const SUBMENU_ROOM = 214;

export function TrayMenuWindow(): ReactElement | null {
  const engine = useAppStore((s) => s.engine);
  const settings = useAppStore((s) => s.settings);
  const profiles = useAppStore((s) => s.profiles);
  const profile = useAppStore(selectActiveProfile);
  const [openCount, setOpenCount] = useState(0);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const disconnect = connectStoreEffect();
    const unlisten = inTauri ? events.trayMenuOpened.listen(() => setOpenCount((n) => n + 1)) : null;
    return () => {
      disconnect();
      void unlisten?.then((f) => f());
    };
  }, []);

  const ready = settings !== null;
  useEffect(() => {
    const menu = menuRef.current;
    if (!ready || !menu || !inTauri) return;
    const observer = new ResizeObserver(() => {
      const box = menu.getBoundingClientRect();
      void commands.trayMenuResize(Math.ceil(box.width) + SUBMENU_ROOM + PAD * 2, Math.ceil(box.height) + PAD * 2);
    });
    observer.observe(menu);
    return () => observer.disconnect();
  }, [ready]);

  if (!settings) return null;
  const close = () => void commands.trayMenuClose();
  return (
    <div
      className="tray-root"
      onPointerDown={(e) => {
        if (e.target === e.currentTarget) close();
      }}
    >
      <div ref={menuRef}>
        <TrayMenuView
          key={openCount}
          state={engine.state}
          profileName={profile?.name ?? "—"}
          profiles={profiles}
          activeProfileId={settings.activeProfileId}
          showHud={settings.showHud}
          startHint={hotkeyLabel(settings.hotkeys.startStop)}
          pauseHint={hotkeyLabel(settings.hotkeys.pauseResume)}
          onAction={(action) => void commands.trayMenuAction(action)}
          onClose={close}
        />
      </div>
    </div>
  );
}

import { TrayMenuView } from "../../features/tray/tray-menu-view";

import type { ReactElement } from "react";

const noop = () => {};

const PROFILES = [
  { id: "quick-farm", name: "Quick farm" },
  { id: "reading-loop", name: "Reading loop" },
  { id: "anti-idle", name: "Anti-idle" },
];

export function TrayMenuDemo(): ReactElement {
  return (
    <div className="tray-demo">
      <TrayMenuView
        state="idle"
        profileName="Quick farm"
        profiles={PROFILES}
        activeProfileId="quick-farm"
        showHud
        startHint="F6"
        pauseHint="F7"
        initialSubmenu
        onAction={noop}
        onClose={noop}
      />
      <TrayMenuView
        state="running"
        profileName="Reading loop"
        profiles={PROFILES}
        activeProfileId="reading-loop"
        showHud={false}
        startHint="F6"
        pauseHint="F7"
        onAction={noop}
        onClose={noop}
      />
    </div>
  );
}

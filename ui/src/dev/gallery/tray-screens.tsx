import { TrayMenuView } from "../../features/tray/tray-menu-view";

import type { ReactElement } from "react";

const noop = () => {};

const PROFILES = [
  { id: "coleta", name: "Coleta rápida" },
  { id: "leitura", name: "Leitura em loop" },
  { id: "anti", name: "Anti-ociosidade" },
];

export function TrayMenuDemo(): ReactElement {
  return (
    <div className="tray-demo">
      <TrayMenuView
        state="idle"
        profileName="Coleta rápida"
        profiles={PROFILES}
        activeProfileId="coleta"
        showHud
        startHint="F6"
        pauseHint="F7"
        initialSubmenu
        onAction={noop}
        onClose={noop}
      />
      <TrayMenuView
        state="running"
        profileName="Leitura em loop"
        profiles={PROFILES}
        activeProfileId="leitura"
        showHud={false}
        startHint="F6"
        pauseHint="F7"
        onAction={noop}
        onClose={noop}
      />
    </div>
  );
}

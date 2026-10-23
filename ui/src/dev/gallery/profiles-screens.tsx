import { DATA_PATH } from "../../features/profiles/profiles-route";
import { ProfilesScreen } from "../../features/profiles/profiles-screen";
import { ThemesScreen } from "../../features/themes/themes-screen";

import { prototypeProfile } from "./fixtures";
import { WindowFrame } from "./window-frame";

import type { ProfilesScreenProps } from "../../features/profiles/profiles-screen";
import type { Profile } from "../../ipc";
import type { ReactElement } from "react";

const noop = () => {};

function handoffProfiles(): Profile[] {
  const base = prototypeProfile();
  const quickFarm: Profile = {
    ...base,
    id: "quick-farm",
    hotkey: { mods: ["ctrl"], key: "1" },
    click: { ...base.click, variation: { ...base.click.variation, enabled: false } },
  };
  const readingLoop: Profile = {
    ...base,
    id: "reading-loop",
    name: "Reading loop",
    mode: "scan",
    hotkey: { mods: ["ctrl"], key: "2" },
    modes: { ...base.modes, scan: { ...base.modes.scan, area: { x: 412, y: 236, w: 776, h: 260 } } },
  };
  const antiIdle: Profile = {
    ...base,
    id: "anti-idle",
    name: "Anti-idle",
    mode: "wiggle",
    hotkey: { mods: ["ctrl"], key: "3" },
  };
  const formTest: Profile = {
    ...base,
    id: "form-test",
    name: "Form test",
    click: { ...base.click, kind: "double" },
    modes: { ...base.modes, click: { ...base.modes.click, position: "cursor" } },
  };
  return [quickFarm, readingLoop, antiIdle, formTest];
}

function Screen(props: Partial<ProfilesScreenProps>) {
  return (
    <WindowFrame width={420} height={640}>
      {() => (
        <ProfilesScreen
          profiles={handoffProfiles()}
          activeId="quick-farm"
          locale="pt-BR"
          dataPath={DATA_PATH}
          recordingProfile={null}
          heldMods={[]}
          conflict={null}
          onBack={noop}
          onMinimize={noop}
          onClose={noop}
          onActivate={noop}
          onCreate={noop}
          onRename={async () => null}
          onDuplicate={noop}
          onExport={noop}
          onDelete={noop}
          onImport={noop}
          onImportFile={noop}
          onReorder={noop}
          onRecordHotkey={noop}
          onClearHotkey={noop}
          onCancelRecord={noop}
          onReplaceHotkey={noop}
          onKeepHotkey={noop}
          {...props}
        />
      )}
    </WindowFrame>
  );
}

export function ProfilesScreenDemo(): ReactElement {
  return <Screen initialMenuFor="reading-loop" />;
}

export function ProfilesEmptyDemo(): ReactElement {
  return <Screen profiles={[]} activeId={null} />;
}

export function ThemesScreenDemo(): ReactElement {
  return (
    <WindowFrame width={420} height={640}>
      {() => (
        <ThemesScreen
          shown="grafite"
          followSystem={false}
          onSelect={noop}
          onToggleFollow={noop}
          onBack={noop}
          onMinimize={noop}
          onClose={noop}
        />
      )}
    </WindowFrame>
  );
}

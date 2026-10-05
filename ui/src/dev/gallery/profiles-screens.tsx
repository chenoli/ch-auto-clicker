import { DATA_PATH } from "../../features/profiles/profiles-route";
import { ProfilesScreen } from "../../features/profiles/profiles-screen";

import { prototypeProfile } from "./fixtures";
import { WindowFrame } from "./window-frame";

import type { ProfilesScreenProps } from "../../features/profiles/profiles-screen";
import type { Profile } from "../../ipc";
import type { ReactElement } from "react";

const noop = () => {};

/** The four profiles of the handoff (Tela 05.A). */
function handoffProfiles(): Profile[] {
  const base = prototypeProfile();
  const coleta: Profile = {
    ...base,
    id: "coleta",
    hotkey: { mods: ["ctrl"], key: "1" },
    click: { ...base.click, variation: { ...base.click.variation, enabled: false } },
  };
  const leitura: Profile = {
    ...base,
    id: "leitura",
    name: "Leitura em loop",
    mode: "scan",
    hotkey: { mods: ["ctrl"], key: "2" },
    modes: { ...base.modes, scan: { ...base.modes.scan, area: { x: 412, y: 236, w: 776, h: 260 } } },
  };
  const anti: Profile = {
    ...base,
    id: "anti",
    name: "Anti-ociosidade",
    mode: "wiggle",
    hotkey: { mods: ["ctrl"], key: "3" },
  };
  const teste: Profile = {
    ...base,
    id: "teste",
    name: "Teste de formulário",
    click: { ...base.click, kind: "double" },
    modes: { ...base.modes, click: { ...base.modes.click, position: "cursor" } },
  };
  return [coleta, leitura, anti, teste];
}

function Screen(props: Partial<ProfilesScreenProps>) {
  return (
    <WindowFrame width={420} height={640}>
      {() => (
        <ProfilesScreen
          profiles={handoffProfiles()}
          activeId="coleta"
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

/** Tela 05.A with the "⋯" menu open on the second card, as in the handoff. */
export function ProfilesScreenDemo(): ReactElement {
  return <Screen initialMenuFor="leitura" />;
}

/** Tela 05.B: the empty state with its drop area. */
export function ProfilesEmptyDemo(): ReactElement {
  return <Screen profiles={[]} activeId={null} />;
}

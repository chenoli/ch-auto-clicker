import { useState } from "react";

import { useTranslation } from "react-i18next";

import { HotkeysScreen } from "../../features/hotkeys/hotkeys-screen";
import { MainWindowView } from "../../features/main/main-window-view";
import { VariationEditorView } from "../../features/variation/variation-editor-view";

import { prototypeProfile, tick } from "./fixtures";
import { WindowFrame } from "./window-frame";

import type { EngineTick, Profile } from "../../ipc";
import type { ReactElement } from "react";

const noop = () => {};

function Window({
  engine,
  errorCode = null,
  countdownTotalMs = 3000,
  initialProfile = prototypeProfile(),
}: {
  engine: EngineTick;
  errorCode?: string | null;
  countdownTotalMs?: number;
  initialProfile?: Profile;
}) {
  const [profile, setProfile] = useState(initialProfile);
  const { i18n } = useTranslation();
  return (
    <WindowFrame width={420}>
      {(onDesiredHeight) => (
        <MainWindowView
          profile={profile}
          locale={i18n.language}
          engine={engine}
          countdownTotalMs={countdownTotalMs}
          shortcut="F6"
          errorCode={errorCode}
          commandError={null}
          onEdit={(mutate) => setProfile(mutate)}
          onPrimary={noop}
          onOpenVariation={noop}
          onMinimize={noop}
          onClose={noop}
          onDesiredHeight={onDesiredHeight}
          monitorWidth={1920}
        />
      )}
    </WindowFrame>
  );
}

export function MainIdleScreen(): ReactElement {
  return <Window engine={tick()} />;
}

function wiggleProfile(clickWhileWiggling: boolean): Profile {
  const p = prototypeProfile();
  return { ...p, mode: "wiggle", modes: { ...p.modes, wiggle: { ...p.modes.wiggle, clickWhileWiggling } } };
}

export function MainWiggleScreen(): ReactElement {
  return (
    <div className="flex gap-10">
      <Window engine={tick()} initialProfile={wiggleProfile(false)} />
      <Window engine={tick()} initialProfile={wiggleProfile(true)} />
    </div>
  );
}

function scanProfile(action: "move" | "move_click"): Profile {
  const p = prototypeProfile();
  return {
    ...p,
    mode: "scan",
    modes: { ...p.modes, scan: { ...p.modes.scan, area: { x: 412, y: 236, w: 776, h: 260 }, action } },
  };
}

export function MainScanScreen(): ReactElement {
  return (
    <div className="flex gap-10">
      <Window engine={tick()} initialProfile={scanProfile("move")} />
      <Window engine={tick()} initialProfile={scanProfile("move_click")} />
    </div>
  );
}

export function MainStatesScreen(): ReactElement {
  return (
    <div className="flex gap-10">
      <Window engine={tick({ state: "countdown", countdownRemainingMs: 2400 })} />
      <Window engine={tick({ state: "running", clicks: 1284, elapsedMs: 128_400, remainingMs: 171_600 })} />
      <Window engine={tick({ state: "paused", clicks: 1284, elapsedMs: 128_400 })} />
      <Window engine={tick({ state: "error" })} errorCode="INPUT_BLOCKED" />
    </div>
  );
}

export function VariationScreen(): ReactElement {
  const { i18n } = useTranslation();
  return (
    <WindowFrame width={560} height={640}>
      {() => (
        <VariationEditorView
          initialBlocks={[1, 35, 70]}
          double={false}
          locale={i18n.language}
          minReliableMs={4}
          onApply={noop}
          onDisable={noop}
          onClose={noop}
          onMinimize={noop}
        />
      )}
    </WindowFrame>
  );
}

const DEFAULT_BINDINGS = {
  startStop: { mods: [], key: "F6" },
  pauseResume: { mods: [], key: "F7" },
  capturePoint: { mods: ["ctrl", "shift"], key: "P" },
  nextProfile: { mods: ["ctrl", "alt"], key: "Right" },
  emergencyStop: { mods: ["ctrl", "alt"], key: "Escape" },
} as const;

function Hotkeys(props: Partial<Parameters<typeof HotkeysScreen>[0]>) {
  return (
    <WindowFrame width={420} height={640}>
      {() => (
        <HotkeysScreen
          bindings={structuredClone(DEFAULT_BINDINGS) as never}
          statuses={[]}
          recording={null}
          heldMods={[]}
          conflict={null}
          onRecord={noop}
          onReplace={noop}
          onKeep={noop}
          onReset={noop}
          onRetry={noop}
          onBack={noop}
          onMinimize={noop}
          onClose={noop}
          {...props}
        />
      )}
    </WindowFrame>
  );
}

export function HotkeysScreenDemo(): ReactElement {
  return <Hotkeys />;
}

export function HotkeysStatesDemo(): ReactElement {
  return (
    <Hotkeys
      recording="startStop"
      conflict={{
        target: { kind: "action", id: "pauseResume" },
        combo: { mods: [], key: "F6" },
        withName: "Iniciar / Parar",
      }}
    />
  );
}

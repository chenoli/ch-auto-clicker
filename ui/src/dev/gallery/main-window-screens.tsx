import { MainWindowView } from "../../features/main/main-window-view";
import { VariationEditorView } from "../../features/variation/variation-editor-view";

import { prototypeProfile, tick } from "./fixtures";
import { WindowFrame } from "./window-frame";

import type { EngineTick } from "../../ipc";
import type { ReactElement } from "react";

const noop = () => {};

function Window({
  engine,
  errorCode = null,
  countdownTotalMs = 3000,
}: {
  engine: EngineTick;
  errorCode?: string | null;
  countdownTotalMs?: number;
}) {
  return (
    <WindowFrame width={420}>
      {(onDesiredHeight) => (
        <MainWindowView
          profile={prototypeProfile()}
          locale="pt-BR"
          engine={engine}
          countdownTotalMs={countdownTotalMs}
          shortcut="F6"
          errorCode={errorCode}
          commandError={null}
          onEdit={noop}
          onPrimary={noop}
          onOpenVariation={noop}
          onMinimize={noop}
          onClose={noop}
          onDesiredHeight={onDesiredHeight}
        />
      )}
    </WindowFrame>
  );
}

/** Tela 01.A in the idle state, with the prototype's profile. */
export function MainIdleScreen(): ReactElement {
  return <Window engine={tick()} />;
}

/** Tela 10: every state of the main button and footer. */
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

/** Tela 02.B with the prototype's blocks and the handoff's provisional 4 ms limit. */
export function VariationScreen(): ReactElement {
  return (
    <WindowFrame width={560} height={640}>
      {() => (
        <VariationEditorView
          initialBlocks={[1, 35, 70]}
          double={false}
          locale="pt-BR"
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

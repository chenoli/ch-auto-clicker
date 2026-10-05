import { commands } from "../ipc";

import { setCommandError, useAppStore } from "./app-store";

import type { EngineState } from "../ipc";

/** What the main button does in each engine state (Tela 10). */
export async function primaryAction(state: EngineState): Promise<void> {
  setCommandError(null);
  switch (state) {
    case "countdown":
    case "running":
      await commands.engineStop();
      return;
    case "paused":
      await commands.engineTogglePause();
      return;
    case "stopping":
      return;
    case "error":
      await commands.engineAckError();
      await start();
      return;
    case "idle":
      await start();
  }
}

async function start(): Promise<void> {
  const result = await commands.engineStart(null);
  if (result.status === "error") setCommandError(result.error.code);
}

export function currentEngineState(): EngineState {
  return useAppStore.getState().engine.state;
}

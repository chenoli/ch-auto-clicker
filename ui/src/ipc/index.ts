import { isTauri } from "@tauri-apps/api/core";

export { commands, events } from "./bindings";
export type * from "./bindings";

export const inTauri = isTauri();

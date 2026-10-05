import { isTauri } from "@tauri-apps/api/core";

export { commands, events } from "./bindings";
export type * from "./bindings";

// False in a plain browser (gallery, Playwright), where the UI must not call into Rust.
export const inTauri = isTauri();

import { describe, expect, it } from "vitest";

import { commands, events, inTauri } from "./index";

describe("ipc", () => {
  it("reports that a plain browser is not Tauri", () => {
    expect(inTauri).toBe(false);
  });

  it("re-exports the generated bindings", () => {
    expect(typeof commands.appBootstrap).toBe("function");
    expect(typeof events.devPing.listen).toBe("function");
  });
});

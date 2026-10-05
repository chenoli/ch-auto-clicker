import { describe, expect, it } from "vitest";

import { hotkeyKeys, hotkeyLabel } from "./format-hotkey";

describe("hotkey formatting", () => {
  it("lists modifiers before the key with friendly names", () => {
    expect(hotkeyKeys({ mods: ["ctrl", "alt"], key: "Escape" })).toEqual(["Ctrl", "Alt", "Esc"]);
    expect(hotkeyLabel({ mods: ["ctrl", "shift"], key: "P" })).toBe("Ctrl Shift P");
    expect(hotkeyLabel({ mods: [], key: "F6" })).toBe("F6");
    expect(hotkeyLabel({ mods: ["win"], key: "MouseX2" })).toBe("Win X2");
  });

  it("shows a dash when unassigned", () => {
    expect(hotkeyLabel(null)).toBe("—");
    expect(hotkeyKeys(undefined)).toEqual([]);
  });
});

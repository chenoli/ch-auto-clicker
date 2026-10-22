import { describe, expect, it } from "vitest";

import { keyNameFromCode, keyNameFromMouseButton } from "./key-name";

describe("keyNameFromCode", () => {
  it.each([
    ["KeyA", "A"],
    ["Digit7", "7"],
    ["Numpad3", "Numpad3"],
    ["F8", "F8"],
    ["F24", "F24"],
    ["ArrowRight", "Right"],
    ["ControlLeft", "Control"],
    ["ShiftRight", "Shift"],
    ["AltLeft", "Alt"],
    ["MetaLeft", "LWin"],
    ["Escape", "Escape"],
    ["Semicolon", "Semicolon"],
    ["NumpadEnter", "Enter"],
  ])("maps %s to %s", (code, name) => expect(keyNameFromCode(code)).toBe(name));

  it.each(["F25", "F0", "IntlBackslash", "NumpadAdd", "Unidentified", ""])("ignores %s", (code) =>
    expect(keyNameFromCode(code)).toBeNull(),
  );
});

describe("keyNameFromMouseButton", () => {
  it("maps middle and side buttons only", () => {
    expect(keyNameFromMouseButton(1)).toBe("MouseMiddle");
    expect(keyNameFromMouseButton(3)).toBe("MouseX1");
    expect(keyNameFromMouseButton(4)).toBe("MouseX2");
    expect(keyNameFromMouseButton(0)).toBeNull();
    expect(keyNameFromMouseButton(2)).toBeNull();
  });
});

import { describe, expect, it } from "vitest";

import { installBrowserKeyGuard, isBrowserShortcut } from "./browser-keys";

const key = (code: string, mods: Partial<Record<"ctrlKey" | "metaKey" | "altKey", boolean>> = {}) => ({
  code,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  ...mods,
});

describe("isBrowserShortcut", () => {
  it.each([
    key("F5"),
    key("F6"),
    key("F12"),
    key("KeyR", { ctrlKey: true }),
    key("KeyP", { metaKey: true }),
    key("ArrowLeft", { altKey: true }),
    key("BrowserBack"),
  ])("blocks %o", (e) => expect(isBrowserShortcut(e)).toBe(true));

  it.each([
    key("KeyR"),
    key("KeyC", { ctrlKey: true }),
    key("KeyV", { ctrlKey: true }),
    key("KeyZ", { ctrlKey: true }),
    key("ArrowLeft"),
    key("Enter"),
  ])("lets %o through", (e) => expect(isBrowserShortcut(e)).toBe(false));
});

describe("installBrowserKeyGuard", () => {
  it("cancels browser shortcuts and back/forward buttons until removed", () => {
    const remove = installBrowserKeyGuard(window);
    const reload = new KeyboardEvent("keydown", { code: "F5", cancelable: true });
    const typing = new KeyboardEvent("keydown", { code: "KeyA", cancelable: true });
    const back = new MouseEvent("mouseup", { button: 3, cancelable: true });
    const left = new MouseEvent("mouseup", { button: 0, cancelable: true });
    [reload, typing, back, left].forEach((e) => window.dispatchEvent(e));
    expect([reload.defaultPrevented, typing.defaultPrevented, back.defaultPrevented, left.defaultPrevented]).toEqual([
      true,
      false,
      true,
      false,
    ]);
    remove();
    const later = new KeyboardEvent("keydown", { code: "F5", cancelable: true });
    window.dispatchEvent(later);
    expect(later.defaultPrevented).toBe(false);
  });
});

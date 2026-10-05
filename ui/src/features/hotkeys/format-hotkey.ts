import i18n from "../../i18n";

import type { Hotkey, Modifier } from "../../ipc";

const MODIFIER_LABELS: Record<Modifier, string> = { ctrl: "Ctrl", alt: "Alt", shift: "Shift", win: "Win" };

const KEY_LABELS: Record<string, string> = {
  Escape: "Esc",
  Right: "→",
  Left: "←",
  Up: "↑",
  Down: "↓",
  MouseX1: "X1",
  MouseX2: "X2",
};

/** Keys of a shortcut as shown in kbd chips: `["Ctrl", "Shift", "P"]`. */
export function hotkeyKeys(hotkey: Hotkey | null | undefined): string[] {
  if (!hotkey) return [];
  // The middle button is a word, so it follows the UI language; the rest are key names.
  const key = hotkey.key === "MouseMiddle" ? i18n.t("click.middle") : (KEY_LABELS[hotkey.key] ?? hotkey.key);
  return [...hotkey.mods.map((m) => MODIFIER_LABELS[m]), key];
}

/** One-line label: "Ctrl Shift P", or "—" when unassigned. */
export function hotkeyLabel(hotkey: Hotkey | null | undefined): string {
  const keys = hotkeyKeys(hotkey);
  return keys.length ? keys.join(" ") : "—";
}

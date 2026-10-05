import type { Hotkey, Modifier } from "../../ipc";

const MODIFIER_LABELS: Record<Modifier, string> = { ctrl: "Ctrl", alt: "Alt", shift: "Shift", win: "Win" };

const KEY_LABELS: Record<string, string> = {
  Escape: "Esc",
  Right: "→",
  Left: "←",
  Up: "↑",
  Down: "↓",
  MouseMiddle: "Meio",
  MouseX1: "X1",
  MouseX2: "X2",
};

/** Keys of a shortcut as shown in kbd chips: `["Ctrl", "Shift", "P"]`. */
export function hotkeyKeys(hotkey: Hotkey | null | undefined): string[] {
  if (!hotkey) return [];
  return [...hotkey.mods.map((m) => MODIFIER_LABELS[m]), KEY_LABELS[hotkey.key] ?? hotkey.key];
}

/** One-line label: "Ctrl Shift P", or "—" when unassigned. */
export function hotkeyLabel(hotkey: Hotkey | null | undefined): string {
  const keys = hotkeyKeys(hotkey);
  return keys.length ? keys.join(" ") : "—";
}

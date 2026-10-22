const SAME = new Set([
  "Backspace",
  "Tab",
  "Enter",
  "Pause",
  "Escape",
  "Space",
  "PageUp",
  "PageDown",
  "End",
  "Home",
  "PrintScreen",
  "Insert",
  "Delete",
  "ScrollLock",
  "Semicolon",
  "Equal",
  "Comma",
  "Minus",
  "Period",
  "Slash",
  "Backquote",
  "Quote",
]);

const RENAMED: Record<string, string> = {
  AltLeft: "Alt",
  AltRight: "Alt",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  ArrowUp: "Up",
  ControlLeft: "Control",
  ControlRight: "Control",
  MetaLeft: "LWin",
  MetaRight: "RWin",
  NumpadEnter: "Enter",
  ShiftLeft: "Shift",
  ShiftRight: "Shift",
};

const MOUSE: Record<number, string> = { 1: "MouseMiddle", 3: "MouseX1", 4: "MouseX2" };

export function keyNameFromCode(code: string): string | null {
  if (SAME.has(code)) return code;
  if (code in RENAMED) return RENAMED[code] ?? null;
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter) return letter[1] ?? null;
  const digit = /^(?:Digit|Numpad)(\d)$/.exec(code);
  if (digit) return code.startsWith("Numpad") ? code : (digit[1] ?? null);
  const fn = /^F(\d{1,2})$/.exec(code);
  if (fn && Number(fn[1]) >= 1 && Number(fn[1]) <= 24) return code;
  return null;
}

export function keyNameFromMouseButton(button: number): string | null {
  return MOUSE[button] ?? null;
}

const CTRL_CODES = new Set(["KeyF", "KeyG", "KeyH", "KeyJ", "KeyN", "KeyO", "KeyP", "KeyR", "KeyS", "KeyT", "KeyU"]);
const ALT_CODES = new Set(["ArrowLeft", "ArrowRight", "Home"]);
const BROWSER_CODES = new Set(["BrowserBack", "BrowserForward", "BrowserHome", "BrowserRefresh", "BrowserSearch"]);
const NAVIGATION_BUTTONS = new Set([3, 4]);

type KeyLike = Pick<KeyboardEvent, "code" | "ctrlKey" | "metaKey" | "altKey">;

export function isBrowserShortcut(e: KeyLike): boolean {
  if (/^F\d{1,2}$/.test(e.code) || BROWSER_CODES.has(e.code)) return true;
  if ((e.ctrlKey || e.metaKey) && CTRL_CODES.has(e.code)) return true;
  return e.altKey && ALT_CODES.has(e.code);
}

export function installBrowserKeyGuard(target: Window = window): () => void {
  const onKey = (e: KeyboardEvent) => {
    if (isBrowserShortcut(e)) e.preventDefault();
  };
  const onMouse = (e: MouseEvent) => {
    if (NAVIGATION_BUTTONS.has(e.button)) e.preventDefault();
  };
  target.addEventListener("keydown", onKey);
  target.addEventListener("mouseup", onMouse);
  return () => {
    target.removeEventListener("keydown", onKey);
    target.removeEventListener("mouseup", onMouse);
  };
}

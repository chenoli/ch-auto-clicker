export type OverlayKeyAction =
  | { kind: "nudge"; dx: number; dy: number }
  | { kind: "add" }
  | { kind: "undo" }
  | { kind: "finish" }
  | { kind: "cancel" }
  | { kind: "none" };

const NUDGES: Record<string, [number, number]> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
};

export function overlayKeyAction(e: { key: string; ctrlKey: boolean; shiftKey: boolean }): OverlayKeyAction {
  const nudge = NUDGES[e.key];
  if (nudge) {
    const step = e.shiftKey ? 10 : 1;
    return { kind: "nudge", dx: nudge[0] * step, dy: nudge[1] * step };
  }
  if (e.key === " ") return { kind: "add" };
  if (e.key === "Enter") return { kind: "finish" };
  if (e.key === "Escape") return { kind: "cancel" };
  if (e.ctrlKey && e.key.toLowerCase() === "z") return { kind: "undo" };
  return { kind: "none" };
}

export function clampToMonitor(
  pos: { x: number; y: number },
  size: { width: number; height: number },
): { x: number; y: number } {
  return {
    x: Math.min(Math.max(0, pos.x), size.width - 1),
    y: Math.min(Math.max(0, pos.y), size.height - 1),
  };
}

export function chcapUrl(path: string, userAgent: string = navigator.userAgent): string {
  return userAgent.includes("Windows") ? `http://chcap.localhost/${path}` : `chcap://localhost/${path}`;
}

export function captureUrl(index: number, userAgent: string = navigator.userAgent): string {
  return chcapUrl(`${index}.bmp`, userAgent);
}

import { useCallback, useState } from "react";

import type { ReactElement, ReactNode } from "react";

const MIN = 640;
const MAX = 900;

interface WindowFrameProps {
  width: number;
  /** Fixed height; omit to let the content drive it like the native window does. */
  height?: number;
  children: (onDesiredHeight: (px: number) => void) => ReactNode;
}

/**
 * Stands in for the OS window in the gallery: border, radius and shadow from the handoff, and the
 * same 640–900 px animated height the native window gets from Rust.
 */
export function WindowFrame({ width, height, children }: WindowFrameProps): ReactElement {
  const [auto, setAuto] = useState(MIN);
  const onDesiredHeight = useCallback((px: number) => setAuto(Math.min(MAX, Math.max(MIN, px))), []);
  return (
    <div
      className="overflow-hidden rounded-window shadow-window window-frame"
      style={{ width, height: height ?? auto }}
    >
      {children(onDesiredHeight)}
    </div>
  );
}

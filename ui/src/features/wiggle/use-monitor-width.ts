import { useEffect, useState } from "react";

/** Physical width of the monitor the window is on; follows moves between monitors and DPI changes. */
export function monitorWidthNow(): number {
  return Math.round(window.screen.width * window.devicePixelRatio) || 1920;
}

export function useMonitorWidth(): number {
  const [width, setWidth] = useState(monitorWidthNow);
  useEffect(() => {
    const update = () => setWidth(monitorWidthNow());
    window.addEventListener("resize", update);
    const dpr = window.matchMedia?.(`(resolution: ${window.devicePixelRatio}dppx)`);
    dpr?.addEventListener?.("change", update);
    return () => {
      window.removeEventListener("resize", update);
      dpr?.removeEventListener?.("change", update);
    };
  }, []);
  return width;
}

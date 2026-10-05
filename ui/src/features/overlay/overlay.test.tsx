import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import "../../i18n";

import { captureUrl, clampToMonitor, overlayKeyAction } from "./overlay-keys";
import { OverlayView } from "./overlay-view";

import type { OverlayState } from "../../ipc";

const key = (k: string, mods: { ctrlKey?: boolean; shiftKey?: boolean } = {}) => ({
  key: k,
  ctrlKey: false,
  shiftKey: false,
  ...mods,
});

describe("overlay keys", () => {
  it("maps arrows, space, enter, esc and ctrl z", () => {
    expect(overlayKeyAction(key("ArrowLeft"))).toEqual({ kind: "nudge", dx: -1, dy: 0 });
    expect(overlayKeyAction(key("ArrowDown", { shiftKey: true }))).toEqual({ kind: "nudge", dx: 0, dy: 10 });
    expect(overlayKeyAction(key(" "))).toEqual({ kind: "add" });
    expect(overlayKeyAction(key("Enter"))).toEqual({ kind: "finish" });
    expect(overlayKeyAction(key("Escape"))).toEqual({ kind: "finish" });
    expect(overlayKeyAction(key("Z", { ctrlKey: true }))).toEqual({ kind: "undo" });
    expect(overlayKeyAction(key("z"))).toEqual({ kind: "none" });
  });

  it("keeps the crosshair on the monitor", () => {
    const size = { width: 1920, height: 1080 };
    expect(clampToMonitor({ x: -3, y: 2000 }, size)).toEqual({ x: 0, y: 1079 });
    expect(clampToMonitor({ x: 5, y: 6 }, size)).toEqual({ x: 5, y: 6 });
  });

  it("builds the capture URL per platform", () => {
    expect(captureUrl(1, "Mozilla/5.0 (Windows NT 10.0)")).toBe("http://chcap.localhost/1.bmp");
    expect(captureUrl(0, "Mozilla/5.0 (X11; Linux)")).toBe("chcap://localhost/0.bmp");
  });
});

const state: OverlayState = {
  monitor: { index: 0, x: 0, y: 0, width: 1920, height: 1080, scale: 1 },
  points: [
    { number: 1, x: 412, y: 236 },
    { number: 2, x: 5000, y: 236 },
  ],
  nextNumber: 3,
  mode: "points",
  area: null,
};
const viewport = { width: 1920, height: 1080 };

describe("OverlayView", () => {
  it("shows the bar and the points of this monitor only", () => {
    const view = render(<OverlayView state={state} image={null} imageUrl={null} cursor={null} viewport={viewport} />);
    expect(screen.getByRole("status")).toHaveTextContent("Clique para adicionar o ponto 03");
    expect(view.container.querySelectorAll(".point-marker")).toHaveLength(1);
    expect(view.container.querySelector(".crosshair-v")).toBeNull();
  });

  it("draws crosshair, coordinates and loupe at the cursor", () => {
    const view = render(
      <OverlayView
        state={{ ...state, monitor: { ...state.monitor, x: -1680, scale: 1.25 } }}
        image={null}
        imageUrl="http://chcap.localhost/0.bmp"
        cursor={{ x: 1000, y: 500 }}
        viewport={viewport}
        fallbackColor="#5C8CEB"
      />,
    );
    expect(view.container.querySelector<HTMLElement>(".crosshair-v")?.style.left).toBe("800px");
    expect(screen.getByText("x -680")).toBeInTheDocument();
    expect(screen.getByText("y 500")).toBeInTheDocument();
    expect(screen.getByText("-680, 500")).toBeInTheDocument();
    expect(screen.getByText("#5C8CEB")).toBeInTheDocument();
    expect(view.container.querySelector(".loupe canvas")).not.toBeNull();
    expect(view.container.querySelector<HTMLElement>(".overlay-root")?.style.backgroundImage).toContain("chcap");
  });
});

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import "../../i18n";

import { clampRect, hudTop, isUsable, moveRect, normalize, physicalSize, resizeRect } from "./area-geometry";
import { AreaView } from "./area-view";

import type { OverlayState } from "../../ipc";

const bounds = { width: 1000, height: 500 };

describe("area geometry", () => {
  it("normalizes rectangles dragged up or left", () => {
    expect(normalize({ x: 100, y: 100, w: -40, h: -20 })).toEqual({ x: 60, y: 80, w: 40, h: 20 });
  });

  it("keeps the rectangle on the monitor", () => {
    expect(clampRect({ x: -10, y: 480, w: 50, h: 50 }, bounds)).toEqual({ x: 0, y: 480, w: 50, h: 20 });
    expect(moveRect({ x: 900, y: 10, w: 200, h: 50 }, 500, -100, bounds)).toEqual({ x: 800, y: 0, w: 200, h: 50 });
  });

  it("resizes from each handle, letting edges cross", () => {
    const r = { x: 100, y: 100, w: 200, h: 100 };
    expect(resizeRect(r, "se", 50, 20, bounds)).toEqual({ x: 100, y: 100, w: 250, h: 120 });
    expect(resizeRect(r, "nw", -50, -50, bounds)).toEqual({ x: 50, y: 50, w: 250, h: 150 });
    expect(resizeRect(r, "n", 0, 150, bounds)).toEqual({ x: 100, y: 200, w: 200, h: 50 });
    expect(resizeRect(r, "e", 2000, 0, bounds)).toEqual({ x: 100, y: 100, w: 900, h: 100 });
    expect(resizeRect(r, "w", 10, 99, bounds)).toEqual({ x: 110, y: 100, w: 190, h: 100 });
  });

  it("measures in physical pixels and enforces the minimum", () => {
    expect(physicalSize({ x: 0, y: 0, w: 388, h: 130 }, 2)).toEqual({ w: 776, h: 260 });
    expect(isUsable({ x: 0, y: 0, w: 8, h: 50 }, 2)).toBe(true);
    expect(isUsable({ x: 0, y: 0, w: 7, h: 50 }, 2)).toBe(false);
    expect(isUsable(null, 1)).toBe(false);
  });

  it("puts the HUD below, or above near the bottom", () => {
    expect(hudTop({ x: 0, y: 100, w: 10, h: 100 }, 500)).toBe(216);
    expect(hudTop({ x: 0, y: 400, w: 10, h: 80 }, 500)).toBe(344);
  });
});

const state: OverlayState = {
  monitor: { index: 0, x: 0, y: 0, width: 1920, height: 1080, scale: 2 },
  mode: "area",
  points: [],
  nextNumber: 1,
  area: null,
  cursor: null,
};

describe("AreaView", () => {
  it("shows only the hint before anything is drawn", () => {
    const view = render(<AreaView state={state} imageUrl={null} rect={null} viewport={bounds} />);
    expect(screen.getByRole("status")).toHaveTextContent("Arraste para selecionar");
    expect(view.container.querySelector(".area-rect")).toBeNull();
  });

  it("renders handles, origin and the HUD, and wires every control", () => {
    const props = {
      onHandleDown: vi.fn(),
      onBodyDown: vi.fn(),
      onConfirm: vi.fn(),
      onMode: vi.fn(),
      onFinish: vi.fn(),
    };
    const view = render(
      <AreaView
        state={state}
        imageUrl="http://chcap.localhost/0.bmp"
        rect={{ x: 240, y: 150, w: 388, h: 130 }}
        viewport={{ width: 960, height: 540 }}
        {...props}
      />,
    );
    expect(view.container.querySelectorAll(".area-handle")).toHaveLength(8);
    expect(screen.getByText("776 × 260")).toBeInTheDocument();
    expect(screen.getByText("X 480 · Y 300")).toBeInTheDocument();
    expect(screen.getByText("480, 300")).toBeInTheDocument();

    fireEvent.pointerDown(view.container.querySelector('[data-handle="se"]') as Element);
    expect(props.onHandleDown).toHaveBeenCalledWith("se", expect.anything());
    fireEvent.pointerDown(view.container.querySelector(".area-rect") as Element);
    expect(props.onBodyDown).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: "Confirmar ↵" }));
    expect(props.onConfirm).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("radio", { name: "Ponto" }));
    expect(props.onMode).toHaveBeenCalledWith("points");
    fireEvent.click(screen.getByRole("radio", { name: "Área" }));
    expect(props.onMode).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: /para cancelar/ }));
    expect(props.onFinish).toHaveBeenCalledOnce();
  });

  it("disables Confirmar for areas below the minimum", () => {
    render(<AreaView state={state} imageUrl={null} rect={{ x: 0, y: 0, w: 4, h: 4 }} viewport={bounds} />);
    expect(screen.getByRole("button", { name: "Confirmar ↵" })).toBeDisabled();
  });
});

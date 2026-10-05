import { createEvent, fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import "../../i18n";

import { manualPoint } from "./points-route";
import { PointsScreen, parseCoordinate, parseDelay, reorder } from "./points-screen";

import type { OverlayMonitor, ScreenPoint } from "../../ipc";
import type { PointsScreenProps } from "./points-screen";

const primary: OverlayMonitor = { index: 0, x: 0, y: 0, width: 1920, height: 1080, scale: 1 };
const left: OverlayMonitor = { index: 1, x: -1680, y: 0, width: 1680, height: 1050, scale: 1 };
const points: ScreenPoint[] = [
  { id: "a", x: 412, y: 236, delayMs: 0 },
  { id: "b", x: 1288, y: 612, delayMs: 120 },
  { id: "c", x: 980, y: 884, delayMs: 0 },
];

function setup(overrides: Partial<PointsScreenProps> = {}) {
  const props: PointsScreenProps = {
    points,
    monitors: [primary],
    layoutChanged: false,
    captureShortcut: "Ctrl Shift P",
    onChange: vi.fn(),
    onCapture: vi.fn(),
    onAddManual: vi.fn(),
    onBack: vi.fn(),
    onMinimize: vi.fn(),
    onClose: vi.fn(),
    ...overrides,
  };
  const view = render(<PointsScreen {...props} />);
  return { props, view };
}

describe("parsers", () => {
  it("accepts integer coordinates, negative ones included", () => {
    expect(parseCoordinate(" 412 ")).toBe(412);
    expect(parseCoordinate("-1680")).toBe(-1680);
    expect(parseCoordinate("")).toBeNull();
    expect(parseCoordinate("4.5")).toBeNull();
    expect(parseCoordinate("abc")).toBeNull();
    expect(parseCoordinate("200000")).toBeNull();
  });

  it("reads delays as shown in the cell and clamps them", () => {
    expect(parseDelay("+120 ms")).toBe(120);
    expect(parseDelay("—")).toBe(0);
    expect(parseDelay("")).toBe(0);
    expect(parseDelay("99999")).toBe(60_000);
    expect(parseDelay("-5")).toBeNull();
    expect(parseDelay("x")).toBeNull();
  });

  it("moves one item to another position", () => {
    expect(reorder(["a", "b", "c"], 0, 2)).toEqual(["b", "c", "a"]);
    expect(reorder(["a", "b", "c"], 2, 0)).toEqual(["c", "a", "b"]);
    expect(reorder(["a"], 5, 0)).toEqual(["a"]);
  });
});

describe("manualPoint", () => {
  it("goes next to the last point, or to the middle of the primary screen", () => {
    expect(manualPoint(points, { monitors: [primary], fingerprint: "" }, "n")).toEqual({
      id: "n",
      x: 1020,
      y: 884,
      delayMs: 0,
    });
    expect(manualPoint([], { monitors: [left, primary], fingerprint: "" }, "n")).toMatchObject({ x: 960, y: 540 });
    expect(manualPoint([], { monitors: [], fingerprint: "" }, "n")).toMatchObject({ x: 960, y: 540 });
  });
});

describe("PointsScreen", () => {
  it("renders the list and the minimap like Tela 02.A", () => {
    const { view } = setup();
    expect(screen.getByText("Pontos na tela")).toBeInTheDocument();
    expect(screen.getByText("3 pontos")).toBeInTheDocument();
    expect(screen.getByLabelText("X 01")).toHaveValue("412");
    expect(screen.getByLabelText("Delay 02")).toHaveValue("+120 ms");
    expect(screen.getByLabelText("Delay 01")).toHaveValue("—");
    expect(view.container.querySelectorAll(".point-marker")).toHaveLength(3);
    expect(view.container.querySelectorAll(".minimap__arrow")).toHaveLength(2);
    expect(screen.getByText("tela 1 · 1920 × 1080")).toBeInTheDocument();
    expect(screen.getByText("Executa 01 → 03 e repete. Delay é somado ao intervalo.")).toBeInTheDocument();
  });

  it("draws every monitor when there is more than one", () => {
    const { view } = setup({ monitors: [left, primary] });
    expect(view.container.querySelectorAll(".minimap__monitor")).toHaveLength(2);
    expect(screen.getByText("2 telas · 3600 × 1080")).toBeInTheDocument();
  });

  it("commits a valid coordinate and restores an invalid one", () => {
    const { props } = setup();
    const x = screen.getByLabelText("X 02");
    fireEvent.change(x, { target: { value: "1300" } });
    fireEvent.keyDown(x, { key: "Enter" });
    expect(props.onChange).toHaveBeenCalledWith([points[0], { ...points[1], x: 1300 }, points[2]]);

    const y = screen.getByLabelText("Y 01");
    fireEvent.change(y, { target: { value: "oops" } });
    fireEvent.blur(y);
    expect(y).toHaveValue("236");
    expect(props.onChange).toHaveBeenCalledTimes(1);
  });

  it("edits the delay", () => {
    const { props } = setup();
    const delay = screen.getByLabelText("Delay 03");
    fireEvent.change(delay, { target: { value: "250" } });
    fireEvent.blur(delay);
    expect(props.onChange).toHaveBeenCalledWith([points[0], points[1], { ...points[2], delayMs: 250 }]);
  });

  it("removes a point", () => {
    const { props } = setup();
    fireEvent.click(screen.getByRole("button", { name: "Remover ponto 02" }));
    expect(props.onChange).toHaveBeenCalledWith([points[0], points[2]]);
  });

  it("reorders by dragging a row onto another", () => {
    const { props, view } = setup();
    const rows = view.container.querySelectorAll(".point-row");
    const [first, , third] = [...rows];
    if (!first || !third) throw new Error("rows missing");
    expect(first).toHaveAttribute("draggable", "false");
    fireEvent.pointerDown(screen.getByRole("button", { name: /Mover ponto 01/ }));
    expect(first).toHaveAttribute("draggable", "true");
    const start = createEvent.dragStart(first);
    Object.defineProperty(start, "dataTransfer", { value: { effectAllowed: "" } });
    fireEvent(first, start);
    fireEvent.dragOver(third);
    expect(third).toHaveAttribute("data-drop-target", "true");
    fireEvent.drop(third);
    expect(props.onChange).toHaveBeenCalledWith([points[1], points[2], points[0]]);
    fireEvent.dragEnd(first);
    expect(third).toHaveAttribute("data-drop-target", "false");
    expect(first).toHaveAttribute("draggable", "false");
  });

  it("reorders with the arrow keys on the handle", () => {
    const { props } = setup();
    const handle = screen.getByRole("button", { name: /Mover ponto 02/ });
    fireEvent.keyDown(handle, { key: "ArrowUp" });
    expect(props.onChange).toHaveBeenLastCalledWith([points[1], points[0], points[2]]);
    fireEvent.keyDown(handle, { key: "ArrowDown" });
    expect(props.onChange).toHaveBeenLastCalledWith([points[0], points[2], points[1]]);
    fireEvent.keyDown(screen.getByRole("button", { name: /Mover ponto 01/ }), { key: "ArrowUp" });
    fireEvent.keyDown(handle, { key: "Tab" });
    expect(props.onChange).toHaveBeenCalledTimes(2);
  });

  it("captures, adds manually and goes back", () => {
    const { props } = setup();
    const capture = screen.getByRole("button", { name: /Capturar na tela/ });
    expect(within(capture).getByText("Ctrl Shift P")).toBeInTheDocument();
    fireEvent.click(capture);
    fireEvent.click(screen.getByRole("button", { name: "+ Manual" }));
    fireEvent.click(screen.getByRole("button", { name: "Voltar" }));
    expect(props.onCapture).toHaveBeenCalled();
    expect(props.onAddManual).toHaveBeenCalled();
    expect(props.onBack).toHaveBeenCalled();
  });

  it("stops adding manual points at the limit", () => {
    const many = Array.from({ length: 64 }, (_, i) => ({ id: String(i), x: i, y: i, delayMs: 0 }));
    setup({ points: many });
    expect(screen.getByRole("button", { name: "+ Manual" })).toBeDisabled();
  });

  it("shows the empty state, the layout warning and command errors", () => {
    setup({ points: [], layoutChanged: true, errorCode: "CAPTURE_FAILED" });
    expect(screen.getByText("0 pontos")).toBeInTheDocument();
    expect(screen.getByText(/Nenhum ponto ainda/)).toBeInTheDocument();
    expect(screen.getByText(/As telas mudaram/)).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("Não foi possível capturar a tela.");
  });
});

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { prototypeProfile } from "../../dev/gallery/fixtures";

import "../../i18n";

import { clampSpacing, scanCycleSecs, scanLength, scanRows, thumbPolyline } from "./scan-math";
import { ScanPanel } from "./scan-panel";

import type { Area, Profile } from "../../ipc";

const AREA: Area = { x: 412, y: 236, w: 776, h: 260 };

describe("scan math", () => {
  it("matches the Rust path: rows, length and the return segment", () => {
    expect(scanRows(AREA, 24)).toBe(11);
    expect(scanRows({ ...AREA, h: 240 }, 24)).toBe(10);
    expect(scanLength(AREA, 24, false)).toBe(11 * 775 + 240);
    expect(scanLength(AREA, 24, true)).toBeCloseTo(11 * 775 + 240 + Math.hypot(775, 240));
    expect(scanLength({ ...AREA, h: 240 }, 24, true)).toBeCloseTo(10 * 775 + 216 + 216);
    expect(scanCycleSecs(AREA, 24, 1200, true)).toBeCloseTo(7.98, 2);
  });

  it("draws the zigzag inside the thumbnail box", () => {
    const pts = thumbPolyline({ x: 0, y: 0, w: 101, h: 17 }, 8).split(" ");
    expect(pts).toEqual(["4,4", "384,4", "384,65", "4,65", "4,126", "384,126"]);
  });

  it("caps the thumbnail path for very tall areas", () => {
    const pts = thumbPolyline({ x: 0, y: 0, w: 100, h: 32_768 }, 8).split(" ");
    expect(pts).toHaveLength(260);
    expect(pts.at(-1)).toMatch(/,12[56](\.\d)?$/);
  });

  it("clamps the spacing to 8–130", () => {
    expect(clampSpacing(4)).toBe(8);
    expect(clampSpacing(134)).toBe(130);
    expect(clampSpacing(24)).toBe(24);
  });
});

function setup(scan: Partial<Profile["modes"]["scan"]> = {}, thumbnailUrl: string | null = null) {
  const base = prototypeProfile();
  let profile: Profile = { ...base, mode: "scan", modes: { ...base.modes, scan: { ...base.modes.scan, ...scan } } };
  const onChange = vi.fn((mutate: (p: Profile) => Profile) => {
    profile = mutate(profile);
  });
  const props = { onSelectArea: vi.fn(), onAdjustClick: vi.fn() };
  const view = render(
    <ScanPanel profile={profile} locale="pt-BR" thumbnailUrl={thumbnailUrl} onChange={onChange} {...props} />,
  );
  return { ...props, view, latest: () => profile };
}

describe("ScanPanel", () => {
  it("asks for an area when there is none", () => {
    const { onSelectArea } = setup();
    expect(screen.getByText("Nenhuma área selecionada")).toBeInTheDocument();
    expect(screen.getByText("Selecione uma área para ver o trajeto")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Selecionar área" }));
    expect(onSelectArea).toHaveBeenCalledOnce();
  });

  it("shows the area, the zigzag and the badge like Tela 01.C", () => {
    const { view } = setup({ area: AREA }, "http://chcap.localhost/thumb/p.bmp");
    expect(screen.getByText("776 × 260 · X 412 Y 236")).toBeInTheDocument();
    expect(screen.getByText("11 linhas · ciclo 8,0 s")).toBeInTheDocument();
    expect(view.container.querySelector("polyline")?.getAttribute("points")).toContain("4,4");
    expect(view.container.querySelector("img")?.getAttribute("src")).toContain("thumb/p.bmp");
  });

  it("edits spacing, speed, action and loop", () => {
    const { latest, onAdjustClick } = setup({ area: AREA });
    fireEvent.click(screen.getByRole("button", { name: "Aumentar espaçamento" }));
    expect(latest().modes.scan.lineSpacingPx).toBe(28);
    fireEvent.click(screen.getByRole("radio", { name: "2400 px/s" }));
    expect(latest().modes.scan.speedPxS).toBe(2400);
    fireEvent.click(screen.getByRole("radio", { name: "Mover e clicar" }));
    expect(latest().modes.scan.action).toBe("move_click");
    fireEvent.click(screen.getByRole("switch", { name: "Recomeçar do topo ao terminar" }));
    expect(latest().modes.scan.loop).toBe(false);
    expect(onAdjustClick).not.toHaveBeenCalled();
  });

  it("shows the inherited rate card when it clicks", () => {
    const { onAdjustClick, latest } = setup({ area: AREA, action: "move_click", lineSpacingPx: 8 });
    expect(screen.getByText("100 ms · 10,0 CPS · esquerdo")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Ajustar ›" }));
    expect(onAdjustClick).toHaveBeenCalledOnce();
    expect(screen.getByRole("button", { name: "Diminuir espaçamento" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Aumentar espaçamento" }));
    expect(latest().modes.scan.lineSpacingPx).toBe(12);
  });
});

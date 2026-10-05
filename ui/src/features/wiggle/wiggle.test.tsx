import { act, fireEvent, render, renderHook, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { prototypeProfile } from "../../dev/gallery/fixtures";

import "../../i18n";

import { monitorWidthNow, useMonitorWidth } from "./use-monitor-width";
import { effectiveAmplitude, formatAmplitude, parseAmplitude, previewHalfWidth } from "./wiggle-math";
import { WigglePanel } from "./wiggle-panel";

import type { Profile } from "../../ipc";

describe("wiggle math", () => {
  it("parses comma or dot and clamps to 0–100 %", () => {
    expect(parseAmplitude("0,05")).toBe(0.05);
    expect(parseAmplitude(" 1.5 % ")).toBe(1.5);
    expect(parseAmplitude("250")).toBe(100);
    expect(parseAmplitude("-3")).toBe(0);
    expect(parseAmplitude("")).toBeNull();
    expect(parseAmplitude("abc")).toBeNull();
  });

  it("formats like the handoff and follows the Rust pixel rule", () => {
    expect(formatAmplitude(0.05, "pt-BR")).toBe("0,05");
    expect(formatAmplitude(0.96, "pt-BR")).toBe("0,96");
    expect(formatAmplitude(12.345, "pt-BR")).toBe("12,35");
    expect(effectiveAmplitude(0.05, 1920)).toBe(1);
    expect(effectiveAmplitude(1, 1920)).toBe(19);
    expect(previewHalfWidth(0.05)).toBeCloseTo(7.76);
    expect(previewHalfWidth(2)).toBe(170);
  });
});

function setup(edits: Partial<Profile["modes"]["wiggle"]> = {}) {
  const base = prototypeProfile();
  let profile: Profile = {
    ...base,
    mode: "wiggle",
    modes: { ...base.modes, wiggle: { ...base.modes.wiggle, ...edits } },
  };
  const onChange = vi.fn((mutate: (p: Profile) => Profile) => {
    profile = mutate(profile);
  });
  const onAdjustClick = vi.fn();
  render(
    <WigglePanel
      profile={profile}
      locale="pt-BR"
      monitorWidth={1920}
      onChange={onChange}
      onAdjustClick={onAdjustClick}
    />,
  );
  return { onChange, onAdjustClick, latest: () => profile };
}

describe("WigglePanel", () => {
  it("renders Tela 01.B with the equivalent in pixels", () => {
    setup();
    expect(screen.getByText(/≈ 0,96 px em 1920/)).toBeInTheDocument();
    expect(screen.getByText(/→ 1 px efetivo/)).toBeInTheDocument();
    expect(screen.getByLabelText("Amplitude em %")).toHaveValue("0,05");
    expect(screen.getByRole("radio", { name: "Média" })).toHaveAttribute("aria-checked", "true");
    expect(screen.queryByText(/Taxa e botão/)).toBeNull();
  });

  it("hides the effective hint once the amplitude is at least one pixel", () => {
    setup({ amplitudePct: 1 });
    expect(screen.getByText(/≈ 19,2 px em 1920/)).toBeInTheDocument();
    expect(screen.queryByText(/efetivo/)).toBeNull();
  });

  it("edits the amplitude with the slider and the text field", () => {
    const { latest } = setup();
    fireEvent.change(screen.getByRole("slider", { name: /Amplitude/ }), { target: { value: "1.25" } });
    expect(latest().modes.wiggle.amplitudePct).toBe(1.25);

    const field = screen.getByLabelText("Amplitude em %");
    fireEvent.change(field, { target: { value: "12,5" } });
    expect(field).toHaveValue("12,5");
    fireEvent.keyDown(field, { key: "Enter" });
    expect(latest().modes.wiggle.amplitudePct).toBe(12.5);

    fireEvent.change(field, { target: { value: "x" } });
    fireEvent.blur(field);
    expect(latest().modes.wiggle.amplitudePct).toBe(12.5);
  });

  it("changes speed and turns clicking on, showing the inherited rate", () => {
    const { latest, onAdjustClick } = setup({ clickWhileWiggling: true });
    fireEvent.click(screen.getByRole("radio", { name: "Rápida" }));
    expect(latest().modes.wiggle.speedHz).toBe(16);
    expect(screen.getByText("100 ms · 10,0 CPS · esquerdo")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Ajustar ›" }));
    expect(onAdjustClick).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("switch", { name: "Clicar enquanto oscila" }));
    expect(latest().modes.wiggle.clickWhileWiggling).toBe(false);
  });

  it("describes double clicks with side buttons", () => {
    const base = prototypeProfile();
    render(
      <WigglePanel
        profile={{
          ...base,
          mode: "wiggle",
          click: { ...base.click, button: "x2", kind: "double" },
          modes: { ...base.modes, wiggle: { ...base.modes.wiggle, clickWhileWiggling: true } },
        }}
        locale="pt-BR"
        monitorWidth={1920}
        onChange={vi.fn()}
        onAdjustClick={vi.fn()}
      />,
    );
    expect(screen.getByText("100 ms · 20,0 CPS · X2 · duplo")).toBeInTheDocument();
  });
});

describe("useMonitorWidth", () => {
  it("reads the physical width and follows resizes", () => {
    vi.spyOn(window.screen, "width", "get").mockReturnValue(1280);
    const dpr = vi.spyOn(window, "devicePixelRatio", "get").mockReturnValue(1.5);
    expect(monitorWidthNow()).toBe(1920);
    const { result, unmount } = renderHook(() => useMonitorWidth());
    expect(result.current).toBe(1920);
    dpr.mockReturnValue(2);
    act(() => {
      window.dispatchEvent(new Event("resize"));
    });
    expect(result.current).toBe(2560);
    unmount();
    vi.restoreAllMocks();
  });
});

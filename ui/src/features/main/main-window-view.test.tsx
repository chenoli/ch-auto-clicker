import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { prototypeProfile, tick } from "../../dev/gallery/fixtures";

import "../../i18n";

import { parseInterval } from "./click-panel";
import { buttonStateFor } from "./main-button";
import { MainWindowView } from "./main-window-view";

import type { Profile } from "../../ipc";
import type { MainWindowViewProps } from "./main-window-view";

function setup(overrides: Partial<MainWindowViewProps> = {}) {
  let profile = overrides.profile ?? prototypeProfile();
  const onEdit = vi.fn((mutate: (p: Profile) => Profile) => {
    profile = mutate(profile);
  });
  const props: MainWindowViewProps = {
    profile,
    locale: "pt-BR",
    engine: tick(),
    countdownTotalMs: 3000,
    shortcut: "F6",
    onEdit,
    onPrimary: vi.fn(),
    onOpenVariation: vi.fn(),
    onMinimize: vi.fn(),
    onClose: vi.fn(),
    ...overrides,
  };
  render(<MainWindowView {...props} />);
  return { props, onEdit, latest: () => profile };
}

describe("MainWindowView", () => {
  it("renders the idle window like Tela 01.A", () => {
    setup();
    expect(screen.getByText("Coleta rápida")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Pronto.*Iniciar/ })).toHaveAttribute("data-state", "idle");
    expect(screen.getByText("10,0")).toBeInTheDocument();
    expect(screen.getByText("1 · 35 · 70 ms em ciclo")).toBeInTheDocument();
    expect(screen.getByText("4 pontos ›")).toBeInTheDocument();
    expect(screen.getByText("00:05:00")).toBeInTheDocument();
    expect(screen.getByText("Parado")).toBeInTheDocument();
  });

  it("wires the title bar, main button and variation link", () => {
    const { props } = setup();
    fireEvent.click(screen.getByRole("button", { name: "Minimizar" }));
    fireEvent.click(screen.getByRole("button", { name: "Fechar" }));
    fireEvent.click(screen.getByRole("button", { name: /Iniciar/ }));
    fireEvent.click(screen.getByRole("button", { name: /Variação/ }));
    expect(props.onMinimize).toHaveBeenCalledOnce();
    expect(props.onClose).toHaveBeenCalledOnce();
    expect(props.onPrimary).toHaveBeenCalledOnce();
    expect(props.onOpenVariation).toHaveBeenCalledOnce();
  });

  it("edits button, side button, click type, position, mode and stop timer", () => {
    const { latest } = setup();
    fireEvent.click(screen.getByRole("radio", { name: "Direito" }));
    expect(latest().click.button).toBe("right");
    fireEvent.click(screen.getByRole("radio", { name: /Lateral/ }));
    fireEvent.click(screen.getByRole("menuitemradio", { name: "X2 · Avançar" }));
    expect(latest().click.button).toBe("x2");
    fireEvent.click(screen.getByRole("radio", { name: "Duplo" }));
    expect(latest().click.kind).toBe("double");
    fireEvent.click(screen.getByRole("radio", { name: /Posição atual do cursor/ }));
    expect(latest().modes.click.position).toBe("cursor");
    fireEvent.click(screen.getByRole("radio", { name: /Pontos na tela/ }));
    expect(latest().modes.click.position).toBe("points");
    fireEvent.click(screen.getByRole("switch", { name: "Parar após" }));
    expect(latest().stopAfter.enabled).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Aumentar" }));
    expect(latest().stopAfter.seconds).toBe(330);
    fireEvent.click(screen.getByRole("radio", { name: /Wiggle/ }));
    expect(latest().mode).toBe("wiggle");
  });

  it("commits the interval on Enter, ignoring junk and clamping", () => {
    const { latest, onEdit } = setup();
    const field = screen.getByRole("textbox", { name: "Intervalo" });
    fireEvent.change(field, { target: { value: "25a0" } });
    expect(screen.getByText("4,0")).toBeInTheDocument();
    fireEvent.keyDown(field, { key: "Enter" });
    expect(latest().click.intervalMs).toBe(250);
    fireEvent.change(field, { target: { value: "" } });
    fireEvent.blur(field);
    expect(onEdit).toHaveBeenCalledTimes(1);
  });

  it("shows countdown, running and error states", () => {
    setup({ engine: tick({ state: "countdown", countdownRemainingMs: 2400 }) });
    expect(screen.getByText("Iniciando em 2,4 s")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("Alterações valem ao reiniciar.")).toBeInTheDocument();
  });

  it("warns when the rate is not met or the screens changed", () => {
    setup({ degraded: "timing" });
    expect(screen.getByRole("note")).toHaveTextContent("ritmo pedido");
  });

  it("explains a run paused by a screen change", () => {
    setup({ degraded: "displayChanged" });
    expect(screen.getByRole("alert")).toHaveTextContent("As telas mudaram");
  });

  it("explains engine and command errors", () => {
    setup({ engine: tick({ state: "error" }), errorCode: "INPUT_BLOCKED", commandError: "NO_AREA" });
    expect(screen.getAllByText("Entrada bloqueada")).toHaveLength(2);
    expect(screen.getByRole("alert")).toHaveTextContent("Selecione a área do scan");
  });

  it("falls back to a generic message for unknown command errors", () => {
    setup({ commandError: "WEIRD" });
    expect(screen.getByRole("alert")).toHaveTextContent("Algo deu errado: WEIRD");
  });

  it("shows the scan panel, with dashes for clicks while it only moves", () => {
    const onSelectArea = vi.fn();
    const base = prototypeProfile();
    const profile = { ...base, mode: "scan" as const };
    const { latest } = setup({ profile, onSelectArea, thumbnailUrl: null });
    expect(screen.getByText("—")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Selecionar área" }));
    expect(onSelectArea).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("radio", { name: "Mover e clicar" }));
    expect(latest().modes.scan.action).toBe("move_click");
  });

  it("goes back to the Click tab from the scan rate card", () => {
    const base = prototypeProfile();
    const profile = {
      ...base,
      mode: "scan" as const,
      modes: { ...base.modes, scan: { ...base.modes.scan, action: "move_click" as const } },
    };
    const { latest } = setup({ profile });
    fireEvent.click(screen.getByRole("button", { name: "Ajustar ›" }));
    expect(latest().mode).toBe("click");
  });

  it("shows the wiggle panel, whose Ajustar link goes back to the Click tab", () => {
    const profile = { ...prototypeProfile(), mode: "wiggle" as const };
    const { latest } = setup({ profile, monitorWidth: 1920 });
    expect(screen.getByText(/≈ 0,96 px em 1920/)).toBeInTheDocument();
    expect(screen.getByText("—")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("switch", { name: "Clicar enquanto oscila" }));
    expect(latest().modes.wiggle.clickWhileWiggling).toBe(true);
  });

  it("switches to the Click tab from the inherited rate card", () => {
    const base = prototypeProfile();
    const profile = {
      ...base,
      mode: "wiggle" as const,
      modes: { ...base.modes, wiggle: { ...base.modes.wiggle, clickWhileWiggling: true } },
    };
    const { latest } = setup({ profile });
    fireEvent.click(screen.getByRole("button", { name: "Ajustar ›" }));
    expect(latest().mode).toBe("click");
  });

  it("shows the shortcut-conflict state while idle and offers both ways out", () => {
    const onReviewHotkeys = vi.fn();
    const onStartAnyway = vi.fn();
    const onOpenSettings = vi.fn();
    setup({ hotkeyError: "F6", onReviewHotkeys, onStartAnyway, onOpenSettings });
    const button = screen.getByRole("button", { name: /Atalho em conflito.*Revisar/ });
    expect(button).toHaveAttribute("data-state", "error");
    expect(screen.getByText("F6", { selector: "kbd" })).toHaveAttribute("data-broken", "true");
    expect(screen.getByText("F6 indisponível")).toBeInTheDocument();
    fireEvent.click(button);
    fireEvent.click(screen.getByRole("button", { name: "Iniciar sem atalho" }));
    fireEvent.click(screen.getByRole("button", { name: "Configurações" }));
    expect(onReviewHotkeys).toHaveBeenCalledOnce();
    expect(onStartAnyway).toHaveBeenCalledOnce();
    expect(onOpenSettings).toHaveBeenCalledOnce();
  });

  it("ignores a broken shortcut while running", () => {
    setup({ hotkeyError: "F6", engine: tick({ state: "running" }) });
    expect(screen.getByRole("button", { name: /Em execução/ })).toHaveAttribute("data-state", "running");
  });

  it("reports the height it needs", () => {
    const onDesiredHeight = vi.fn();
    setup({ onDesiredHeight });
    expect(onDesiredHeight).toHaveBeenCalled();
  });
});

describe("helpers", () => {
  it("parses intervals", () => {
    expect(parseInterval("abc")).toBeNull();
    expect(parseInterval("0")).toBe(1);
    expect(parseInterval("999999")).toBe(60_000);
  });

  it("maps engine states to button states", () => {
    expect(buttonStateFor("stopping")).toBe("running");
    expect(buttonStateFor("idle")).toBe("idle");
    expect(buttonStateFor("paused")).toBe("paused");
  });
});

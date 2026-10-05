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
  let profile = prototypeProfile();
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

  it("explains engine and command errors", () => {
    setup({ engine: tick({ state: "error" }), errorCode: "INPUT_BLOCKED", commandError: "MODE_NOT_AVAILABLE" });
    expect(screen.getAllByText("Entrada bloqueada")).toHaveLength(2);
    expect(screen.getByRole("alert")).toHaveTextContent("Este modo ainda não está disponível");
  });

  it("falls back to a generic message for unknown command errors", () => {
    setup({ commandError: "WEIRD" });
    expect(screen.getByRole("alert")).toHaveTextContent("Algo deu errado: WEIRD");
  });

  it("shows a placeholder for modes from later phases and dashes for move-only modes", () => {
    const profile = { ...prototypeProfile(), mode: "scan" as const };
    setup({ profile });
    expect(screen.getByText("O modo Scan chega em uma próxima fase.")).toBeInTheDocument();
    expect(screen.getByText("—")).toBeInTheDocument();
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

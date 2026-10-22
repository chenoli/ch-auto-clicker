import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import "../../i18n";

import { HudView } from "./hud-view";

import type { HudViewProps } from "./hud-view";

function setup(overrides: Partial<HudViewProps> = {}) {
  const props: HudViewProps = {
    state: "running",
    locale: "pt-BR",
    profileName: "Coleta rápida",
    clicks: 1284,
    countdownRemainingMs: null,
    remainingMs: 272_000,
    startStop: "F6",
    pauseResume: "F7",
    onDragStart: vi.fn(),
    ...overrides,
  };
  const view = render(<HudView {...props} />);
  return { props, view };
}

describe("HudView", () => {
  it("counts down with the profile name (04.A)", () => {
    setup({ state: "countdown", countdownRemainingMs: 2400, remainingMs: null });
    const pill = screen.getByRole("status");
    expect(pill).toHaveAttribute("data-state", "countdown");
    expect(pill).toHaveTextContent("Iniciando");
    expect(pill).toHaveTextContent("3");
    expect(pill).toHaveTextContent("Coleta rápida");
    expect(pill).toHaveTextContent("F6cancelar");
  });

  it("shows clicks and the remaining time while running (04.B)", () => {
    setup();
    const pill = screen.getByRole("status");
    expect(pill).toHaveTextContent("1.284");
    expect(pill).toHaveTextContent("Resta04:32");
    expect(pill).toHaveTextContent("F6parar");
  });

  it("offers F7 to resume when paused (04.C) and hides the remaining time without a timer", () => {
    setup({ state: "paused", remainingMs: null });
    const pill = screen.getByRole("status");
    expect(pill).toHaveTextContent("Pausado");
    expect(pill).toHaveTextContent("F7retomar");
    expect(pill).not.toHaveTextContent("Resta");
  });

  it("fades on hover, drags only with Ctrl and shows the hint", () => {
    const { props, view } = setup({ hover: true });
    const pill = screen.getByRole("status");
    expect(pill).toHaveAttribute("data-hover", "true");
    expect(screen.getByText("Segure Ctrl para mover")).toBeInTheDocument();
    fireEvent.pointerDown(pill);
    expect(props.onDragStart).not.toHaveBeenCalled();
    view.rerender(<HudView {...props} hover draggable />);
    fireEvent.pointerDown(screen.getByRole("status"));
    expect(props.onDragStart).toHaveBeenCalledOnce();
  });

  it("swaps the hint for a warning while an administrator window blocks the clicks", () => {
    const { props, view } = setup({ blockedByAdmin: true });
    expect(screen.getByRole("alert")).toHaveTextContent("Bloqueado por janela de admin");
    expect(screen.queryByText("Segure Ctrl para mover")).not.toBeInTheDocument();
    view.rerender(<HudView {...props} state="paused" />);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("renders nothing while idle", () => {
    const { view } = setup({ state: "idle" });
    expect(view.container).toBeEmptyDOMElement();
  });
});

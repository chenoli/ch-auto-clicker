import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import "../../i18n";

import { TrayMenuView, stateKey } from "./tray-menu-view";

import type { TrayMenuViewProps } from "./tray-menu-view";

const PROFILES = [
  { id: "a", name: "Coleta rápida" },
  { id: "b", name: "Leitura em loop" },
];

function setup(overrides: Partial<TrayMenuViewProps> = {}) {
  const props: TrayMenuViewProps = {
    state: "idle",
    profileName: "Coleta rápida",
    profiles: PROFILES,
    activeProfileId: "a",
    showHud: true,
    startHint: "F6",
    pauseHint: "F7",
    onAction: vi.fn(),
    onClose: vi.fn(),
    ...overrides,
  };
  render(<TrayMenuView {...props} />);
  return { props, menu: screen.getAllByRole("menu")[0] as HTMLElement };
}

const item = (name: RegExp | string) => screen.getByRole("menuitem", { name });
const focusedText = () => document.querySelector('.tray-item[data-focused="true"]')?.textContent;

describe("TrayMenuView", () => {
  it("shows the profile, the state and the 09.A items", () => {
    setup();
    expect(screen.getByText("PARADO")).toBeInTheDocument();
    expect(item(/Iniciar/)).toHaveTextContent("F6");
    expect(item(/Pausar/)).toBeDisabled();
    expect(screen.getByRole("menuitemcheckbox", { name: /Mostrar HUD/ })).toHaveAttribute("aria-checked", "true");
    expect(item(/Me pague um café/)).toHaveAttribute("data-tone", "support");
    expect(item("Sair")).toHaveAttribute("data-tone", "muted");
  });

  it("offers stop and resume while active", () => {
    setup({ state: "paused", showHud: false });
    expect(screen.getByText("PAUSADO")).toBeInTheDocument();
    expect(item(/Parar/)).toBeInTheDocument();
    expect(item(/Retomar/)).toBeEnabled();
    expect(screen.getByRole("menuitemcheckbox", { name: /Mostrar HUD/ })).toHaveAttribute("aria-checked", "false");
  });

  it("maps stopping to the idle label", () => {
    expect(stateKey("stopping")).toBe("idle");
    expect(stateKey("error")).toBe("error");
  });

  it("runs actions by click and closes the submenu on other items", () => {
    const { props } = setup();
    fireEvent.click(item(/Iniciar/));
    fireEvent.click(screen.getByRole("menuitemcheckbox", { name: /Mostrar HUD/ }));
    fireEvent.click(item("Sair"));
    expect(props.onAction).toHaveBeenNthCalledWith(1, "start");
    expect(props.onAction).toHaveBeenNthCalledWith(2, "hud");
    expect(props.onAction).toHaveBeenNthCalledWith(3, "quit");
    fireEvent.pointerEnter(item(/Perfil/));
    const sub = screen.getByRole("menu", { name: "Perfil" });
    fireEvent.pointerEnter(within(sub).getByRole("menuitemradio", { name: /Leitura/ }));
    fireEvent.click(within(sub).getByRole("menuitemradio", { name: /Leitura/ }));
    expect(props.onAction).toHaveBeenLastCalledWith("profile:b");
    fireEvent.pointerEnter(item("Configurações"));
    expect(screen.queryByRole("menu", { name: "Perfil" })).toBeNull();
    fireEvent.click(item(/Perfil/));
    expect(screen.getByRole("menu", { name: "Perfil" })).toBeInTheDocument();
  });

  it("navigates with the keyboard, skipping disabled items", () => {
    const { props, menu } = setup();
    expect(document.activeElement).toBe(menu);
    expect(focusedText()).toContain("Iniciar");
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    expect(focusedText()).toContain("Perfil");
    fireEvent.keyDown(menu, { key: "ArrowUp" });
    fireEvent.keyDown(menu, { key: "ArrowUp" });
    expect(focusedText()).toBe("Sair");
    fireEvent.keyDown(menu, { key: "Enter" });
    expect(props.onAction).toHaveBeenLastCalledWith("quit");
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    fireEvent.keyDown(menu, { key: " " });
    expect(props.onAction).toHaveBeenLastCalledWith("start");
    fireEvent.keyDown(menu, { key: "Tab" });
    fireEvent.keyDown(menu, { key: "Escape" });
    expect(props.onClose).toHaveBeenCalledOnce();
  });

  it("opens the profile submenu with the left arrow and picks with Enter", () => {
    const { props, menu } = setup();
    fireEvent.keyDown(menu, { key: "ArrowLeft" });
    expect(screen.queryByRole("menu", { name: "Perfil" })).toBeNull();
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    fireEvent.keyDown(menu, { key: "ArrowLeft" });
    const sub = screen.getByRole("menu", { name: "Perfil" });
    expect(within(sub).getByRole("menuitemradio", { name: /Coleta/ })).toHaveAttribute("data-focused", "true");
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    fireEvent.keyDown(menu, { key: "ArrowUp" });
    fireEvent.keyDown(menu, { key: "Enter" });
    expect(props.onAction).toHaveBeenLastCalledWith("profile:b");
    fireEvent.keyDown(menu, { key: "Tab" });
    fireEvent.keyDown(menu, { key: "Escape" });
    expect(screen.queryByRole("menu", { name: "Perfil" })).toBeNull();
    expect(props.onClose).not.toHaveBeenCalled();
    fireEvent.keyDown(menu, { key: "ArrowLeft" });
    fireEvent.keyDown(menu, { key: "ArrowRight" });
    expect(screen.queryByRole("menu", { name: "Perfil" })).toBeNull();
  });

  it("copes with an empty profile list", () => {
    const { props, menu } = setup({ profiles: [], activeProfileId: null, initialSubmenu: true });
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    fireEvent.keyDown(menu, { key: "ArrowUp" });
    fireEvent.keyDown(menu, { key: "Enter" });
    expect(props.onAction).not.toHaveBeenCalled();
  });
});

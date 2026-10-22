import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import "../../i18n";

import { SettingsScreen } from "./settings-screen";

import type { Capabilities, Settings } from "../../ipc";
import type { SettingsScreenProps } from "./settings-screen";

const settings = {
  schemaVersion: 1,
  locale: "pt-BR",
  theme: "grafite",
  followSystemTheme: false,
  autostart: false,
  minimizeToTray: true,
  showHud: true,
  closeAction: "minimize",
  countdownSec: 3,
  inputBackend: "sendinput",
  activeProfileId: "p",
  profileOrder: ["p"],
  hotkeys: {},
  hud: { x: null, y: null },
  window: { x: null, y: null },
  antiCheatAcknowledged: false,
} as unknown as Settings;

const capabilities = {
  minReliableIntervalMs: 2,
  measured: true,
  timerKind: "high-resolution",
  mmcss: true,
  measuredAtMs: 1,
  fingerprint: "x",
  results: [
    { intervalMs: 1, p99Us: 1890.4, missed: 3, reliable: false },
    { intervalMs: 2, p99Us: 12.2, missed: 0, reliable: true },
  ],
} as Capabilities;

function setup(overrides: Partial<SettingsScreenProps> = {}) {
  const props: SettingsScreenProps = {
    settings,
    version: "0.1.0",
    capabilities,
    themeLabel: "Grafite",
    onChange: vi.fn(),
    onLocale: vi.fn(),
    onOpenThemes: vi.fn(),
    onOpenHotkeys: vi.fn(),
    onSupport: vi.fn(),
    onMeasure: vi.fn(),
    onBack: vi.fn(),
    onMinimize: vi.fn(),
    onClose: vi.fn(),
    ...overrides,
  };
  render(<SettingsScreen {...props} />);
  return props;
}

describe("SettingsScreen", () => {
  it("saves every preference of Tela 08 at once", () => {
    const props = setup();
    fireEvent.click(screen.getByRole("switch", { name: "Iniciar com o Windows" }));
    expect(props.onChange).toHaveBeenLastCalledWith({ ...settings, autostart: true });
    fireEvent.click(screen.getByRole("switch", { name: "Minimizar para a bandeja" }));
    expect(props.onChange).toHaveBeenLastCalledWith({ ...settings, minimizeToTray: false });
    fireEvent.click(screen.getByRole("switch", { name: "Mostrar HUD ao rodar" }));
    expect(props.onChange).toHaveBeenLastCalledWith({ ...settings, showHud: false });
    fireEvent.click(screen.getByRole("button", { name: "Mais segundos" }));
    expect(props.onChange).toHaveBeenLastCalledWith({ ...settings, countdownSec: 4 });
    fireEvent.click(screen.getByRole("button", { name: "Menos segundos" }));
    expect(props.onChange).toHaveBeenLastCalledWith({ ...settings, countdownSec: 2 });
    fireEvent.click(screen.getByRole("radio", { name: "Sair" }));
    expect(props.onChange).toHaveBeenLastCalledWith({ ...settings, closeAction: "quit" });
    fireEvent.click(screen.getByRole("radio", { name: "English" }));
    expect(props.onLocale).toHaveBeenCalledWith("en-US");
  });

  it("turns on running as administrator and offers to restart until it is elevated", () => {
    const props = setup();
    expect(screen.queryByRole("button", { name: /Reiniciar como administrador/ })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("switch", { name: "Sempre abrir como administrador" }));
    expect(props.onChange).toHaveBeenLastCalledWith({ ...settings, runAsAdmin: true });
  });

  it("shows the restart link only while the option is on and the app is not elevated", () => {
    const props = setup({ settings: { ...settings, runAsAdmin: true }, onRestartAsAdmin: vi.fn() });
    fireEvent.click(screen.getByRole("button", { name: /Reiniciar como administrador/ }));
    expect(props.onRestartAsAdmin).toHaveBeenCalledOnce();
  });

  it("says when it already runs as administrator", () => {
    setup({ settings: { ...settings, runAsAdmin: true }, elevated: true });
    expect(screen.getByText("Rodando como administrador agora")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Reiniciar como administrador/ })).not.toBeInTheDocument();
  });

  it("limits the countdown to 0–10 s", () => {
    setup({ settings: { ...settings, countdownSec: 10 } });
    expect(screen.getByRole("button", { name: "Mais segundos" })).toBeDisabled();
  });

  it("links to Atalhos and Tema, opens Ko-fi and shows the version", () => {
    const props = setup();
    fireEvent.click(screen.getByRole("button", { name: /^Atalhos/ }));
    fireEvent.click(screen.getByRole("button", { name: /Tema.*Grafite ›/ }));
    fireEvent.click(screen.getByRole("button", { name: "Apoiar ↗" }));
    expect(props.onOpenHotkeys).toHaveBeenCalledOnce();
    expect(props.onOpenThemes).toHaveBeenCalledOnce();
    expect(props.onSupport).toHaveBeenCalledOnce();
    expect(screen.getByText("CH Auto Clicker 0.1.0 · Chenoli Labs")).toBeInTheDocument();
    expect(screen.getByRole("note")).toHaveTextContent("Jogos online com anti-cheat");
  });

  it("shows the measured limits under Avançado and measures again", () => {
    const props = setup();
    const toggle = screen.getByRole("button", { name: /Avançado/ });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(toggle);
    expect(screen.getAllByText("2 ms")).toHaveLength(2);
    expect(screen.getByText("1890 µs")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Medir novamente" }));
    expect(props.onMeasure).toHaveBeenCalledOnce();
  });

  it("shows every update state in the footer", () => {
    const onCheckUpdate = vi.fn();
    const onInstallUpdate = vi.fn();
    const props = setup({ update: { kind: "idle" }, onCheckUpdate, onInstallUpdate });
    fireEvent.click(screen.getByRole("button", { name: "Verificar atualizações" }));
    expect(onCheckUpdate).toHaveBeenCalledOnce();
    for (const [update, text] of [
      [{ kind: "checking" }, "Verificando…"],
      [{ kind: "installing" }, "Baixando a atualização…"],
    ] as const) {
      setup({ ...props, update });
      expect(screen.getAllByText(text).length).toBeGreaterThan(0);
    }
    setup({ ...props, update: { kind: "latest" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Você está na versão mais recente" })[0] as HTMLElement);
    expect(onCheckUpdate).toHaveBeenCalledTimes(2);
    setup({ ...props, update: { kind: "available", version: "0.1.1" } });
    fireEvent.click(screen.getByRole("button", { name: "Instalar a 0.1.1 e reiniciar" }));
    expect(onInstallUpdate).toHaveBeenCalledOnce();
  });

  it("says when nothing was measured yet, while measuring and on errors", () => {
    setup({
      capabilities: { ...capabilities, measured: false, minReliableIntervalMs: 4, results: [] },
      measuring: true,
      errorCode: "AUTOSTART_FAILED",
    });
    fireEvent.click(screen.getByRole("button", { name: /Avançado/ }));
    expect(screen.getByText("4 ms (ainda não medido)")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Medindo…" })).toBeDisabled();
    expect(screen.getByRole("alert")).toHaveTextContent("inicialização com o Windows");
  });
});

import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { prototypeProfile, tick } from "../dev/gallery/fixtures";

import "../i18n";

import type { Profile } from "../ipc";

const mocks = vi.hoisted(() => ({
  window: { minimize: vi.fn(), close: vi.fn() },
  commands: {
    appBootstrap: vi.fn(),
    profileUpdate: vi.fn(),
    engineStart: vi.fn(),
    windowOpenVariation: vi.fn(),
    windowSetContentHeight: vi.fn(),
    hotkeyResetDefaults: vi.fn(),
    hotkeyRetry: vi.fn(),
    hotkeyBeginRecord: vi.fn(),
    hotkeyCancelRecord: vi.fn(),
  },
}));

vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => mocks.window }));

vi.mock("../ipc", () => {
  const event = { listen: vi.fn(async () => () => {}) };
  return {
    inTauri: true,
    commands: mocks.commands,
    events: {
      engineTick: event,
      engineStatus: event,
      settingsChanged: event,
      profilesChanged: event,
      capabilities: event,
      hotkeyStatusChanged: event,
      hotkeyActionFailed: event,
      hotkeyRecording: event,
    },
  };
});

const { MainWindow } = await import("./main/main-window");
const { VariationWindow } = await import("./variation/variation-window");
const { useAppStore } = await import("../store/app-store");

function bootstrap(profile: Profile = prototypeProfile()) {
  return {
    version: "0.1.0",
    dpiAwareness: "PerMonitorV2",
    settings: {
      activeProfileId: profile.id,
      locale: "pt-BR",
      countdownSec: 3,
      hotkeys: {
        startStop: { mods: [], key: "F6" },
        pauseResume: { mods: [], key: "F7" },
        capturePoint: null,
        nextProfile: null,
        emergencyStop: { mods: ["ctrl", "alt"], key: "Escape" },
      },
    },
    profiles: [profile],
    engine: tick(),
    capabilities: { minReliableIntervalMs: 2 },
    hotkeys: [],
  };
}

beforeEach(() => {
  useAppStore.setState({ ready: false, profiles: [], settings: null, engine: tick(), commandError: null });
  mocks.commands.appBootstrap.mockResolvedValue(bootstrap());
});

afterEach(() => vi.resetAllMocks());

describe("MainWindow", () => {
  it("loads the active profile and drives the window", async () => {
    mocks.commands.engineStart.mockResolvedValue({ status: "ok", data: null });
    mocks.commands.windowOpenVariation.mockResolvedValue({ status: "ok", data: null });
    render(<MainWindow />);
    expect(await screen.findByText("Coleta rápida")).toBeInTheDocument();
    expect(screen.getByText("F6")).toBeInTheDocument();
    expect(mocks.commands.windowSetContentHeight).toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: /Iniciar/ }));
    expect(mocks.commands.engineStart).toHaveBeenCalledWith(null);
    fireEvent.click(screen.getByRole("button", { name: /Variação/ }));
    expect(mocks.commands.windowOpenVariation).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: "Minimizar" }));
    fireEvent.click(screen.getByRole("button", { name: "Fechar" }));
    expect(mocks.window.minimize).toHaveBeenCalledOnce();
    expect(mocks.window.close).toHaveBeenCalledOnce();
  });

  it("navigates to settings and shortcuts and back", async () => {
    render(<MainWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "Configurações" }));
    expect(mocks.commands.windowSetContentHeight).toHaveBeenLastCalledWith(640);
    fireEvent.click(screen.getByRole("button", { name: /Atalhos ›/ }));
    expect(screen.getByRole("heading", { name: "Atalhos" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Pausar \/ Retomar/ }));
    expect(mocks.commands.hotkeyBeginRecord).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: "Restaurar padrão" }));
    expect(mocks.commands.hotkeyResetDefaults).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: "Voltar" }));
    expect(mocks.commands.hotkeyCancelRecord).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: "Voltar" }));
    fireEvent.click(screen.getByRole("button", { name: "Minimizar" }));
    fireEvent.click(screen.getByRole("button", { name: "Fechar" }));
    expect(await screen.findByText("Coleta rápida")).toBeInTheDocument();
  });

  it("shows the conflict state when F6 could not be registered", async () => {
    mocks.commands.appBootstrap.mockResolvedValue({
      ...bootstrap(),
      hotkeys: [
        {
          target: { kind: "action", id: "startStop" },
          combo: { mods: [], key: "F6" },
          ok: false,
          errorCode: "HOTKEY_TAKEN",
        },
      ],
    });
    render(<MainWindow />);
    fireEvent.click(await screen.findByRole("button", { name: /Revisar/ }));
    expect(screen.getByRole("heading", { name: "Atalhos" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));
    expect(mocks.commands.hotkeyRetry).toHaveBeenCalledOnce();
  });

  it("saves edits through the profile sync", async () => {
    mocks.commands.profileUpdate.mockImplementation(async (p: Profile) => ({ status: "ok", data: p }));
    render(<MainWindow />);
    fireEvent.click(await screen.findByRole("radio", { name: "Duplo" }));
    expect(mocks.commands.profileUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ click: expect.objectContaining({ kind: "double" }) }),
      expect.any(String),
    );
  });

  it("explains a failed connection instead of showing a blank window", async () => {
    mocks.commands.appBootstrap.mockRejectedValue(new Error("ipc down"));
    render(<MainWindow />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Não foi possível carregar");
    expect(screen.getByRole("alert")).toHaveTextContent("ipc down");
  });

  it("renders nothing until the state is loaded", () => {
    mocks.commands.appBootstrap.mockReturnValue(new Promise(() => {}));
    const { container } = render(<MainWindow />);
    expect(container).toBeEmptyDOMElement();
  });
});

describe("VariationWindow", () => {
  it("applies the cycle and closes only after Rust accepted it", async () => {
    mocks.commands.profileUpdate.mockImplementation(async (p: Profile) => ({ status: "ok", data: p }));
    render(<VariationWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "Aplicar ao perfil" }));
    await act(async () => {});
    expect(mocks.commands.profileUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        click: expect.objectContaining({ variation: { enabled: true, blocksMs: [1, 35, 70] } }),
      }),
      "2026-10-04T00:00:00Z",
    );
    expect(mocks.window.close).toHaveBeenCalledOnce();
  });

  it("keeps the window open and explains a refused save", async () => {
    mocks.commands.profileUpdate.mockResolvedValue({ status: "error", error: { code: "STALE_PROFILE", message: "" } });
    render(<VariationWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "Desativar variação" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("alterado em outra janela");
    expect(mocks.window.close).not.toHaveBeenCalled();
    expect(mocks.commands.profileUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        click: expect.objectContaining({ variation: expect.objectContaining({ enabled: false }) }),
      }),
      expect.any(String),
    );
  });

  it("explains a failed connection", async () => {
    mocks.commands.appBootstrap.mockRejectedValue(new Error("boom"));
    render(<VariationWindow />);
    expect(await screen.findByRole("alert")).toHaveTextContent("boom");
  });

  it("uses the measured reliable minimum and minimizes", async () => {
    render(<VariationWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "Minimizar" }));
    expect(mocks.window.minimize).toHaveBeenCalledOnce();
    expect(screen.getByRole("note")).toHaveTextContent("Intervalos abaixo de 2 ms");
  });
});

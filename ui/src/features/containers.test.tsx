import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { prototypeProfile, tick } from "../dev/gallery/fixtures";

import "../i18n";

import type { Profile } from "../ipc";

const mocks = vi.hoisted(() => ({
  window: { minimize: vi.fn(), close: vi.fn(), setFocus: vi.fn() },
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
    screenLayout: vi.fn(),
    overlayPickPoints: vi.fn(),
    overlayState: vi.fn(),
    overlayAddPoint: vi.fn(),
    overlayUndo: vi.fn(),
    overlayFinish: vi.fn(),
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
const { OverlayWindow } = await import("./overlay/overlay-window");
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

describe("PointsRoute", () => {
  const layout = {
    monitors: [{ index: 0, x: 0, y: 0, width: 1920, height: 1080, scale: 1 }],
    fingerprint: "1mon:1920x1080@0,0",
  };

  async function openPoints(profile: Profile = prototypeProfile()) {
    mocks.commands.appBootstrap.mockResolvedValue(bootstrap(profile));
    mocks.commands.screenLayout.mockResolvedValue(layout);
    mocks.commands.profileUpdate.mockImplementation(async (p: Profile) => ({ status: "ok", data: p }));
    render(<MainWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "4 pontos ›" }));
    expect(await screen.findByRole("heading", { name: "Pontos na tela" })).toBeInTheDocument();
  }

  it("opens from the click panel, edits points and goes back", async () => {
    await openPoints();
    expect(mocks.commands.screenLayout).toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "+ Manual" }));
    expect(mocks.commands.profileUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        modes: expect.objectContaining({
          click: expect.objectContaining({
            layoutFingerprint: layout.fingerprint,
            points: expect.arrayContaining([expect.objectContaining({ x: 340, y: 820 })]),
          }),
        }),
      }),
      expect.any(String),
    );
    fireEvent.click(screen.getByRole("button", { name: "Voltar" }));
    expect(await screen.findByText("Coleta rápida")).toBeInTheDocument();
  });

  it("warns when the screens changed since the capture", async () => {
    const profile = prototypeProfile();
    profile.modes.click.layoutFingerprint = "2mon:old";
    await openPoints(profile);
    expect(await screen.findByText(/As telas mudaram/)).toBeInTheDocument();
  });

  it("opens the capture overlay and explains a failure", async () => {
    await openPoints();
    mocks.commands.overlayPickPoints.mockResolvedValue({
      status: "error",
      error: { code: "CAPTURE_FAILED", message: "gdi" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Capturar na tela/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Não foi possível capturar a tela.");
    mocks.commands.overlayPickPoints.mockResolvedValue({ status: "ok", data: [] });
    fireEvent.click(screen.getByRole("button", { name: /Capturar na tela/ }));
    await act(async () => {});
    expect(screen.queryByRole("alert")).toBeNull();
  });
});

describe("OverlayWindow", () => {
  const state = {
    monitor: { index: 0, x: 0, y: 0, width: 1024, height: 768, scale: 1 },
    points: [{ number: 1, x: 100, y: 100 }],
    nextNumber: 2,
  };

  async function openOverlay() {
    mocks.commands.overlayState.mockResolvedValue({ status: "ok", data: state });
    mocks.commands.overlayAddPoint.mockResolvedValue({
      status: "ok",
      data: { ...state, points: [...state.points, { number: 2, x: 10, y: 20 }], nextNumber: 3 },
    });
    const view = render(<OverlayWindow />);
    expect(await screen.findByText(/Clique para adicionar o ponto/)).toBeInTheDocument();
    return view;
  }

  it("adds points by click and by keyboard, undoes and finishes", async () => {
    const view = await openOverlay();
    const root = view.container.firstElementChild as HTMLElement;
    fireEvent.pointerEnter(root);
    expect(mocks.window.setFocus).toHaveBeenCalled();
    fireEvent.pointerMove(root, { clientX: 10, clientY: 20 });
    expect(screen.getByText("10, 20")).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(screen.getByText("11, 20")).toBeInTheDocument();
    fireEvent.keyDown(window, { key: " " });
    expect(mocks.commands.overlayAddPoint).toHaveBeenLastCalledWith(0, 11, 20);
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("ponto 03"));

    fireEvent.pointerDown(root, { button: 0, clientX: 30, clientY: 40 });
    expect(mocks.commands.overlayAddPoint).toHaveBeenLastCalledWith(0, 30, 40);
    fireEvent.pointerDown(root, { button: 2, clientX: 1, clientY: 1 });
    expect(mocks.commands.overlayAddPoint).toHaveBeenCalledTimes(2);

    fireEvent.keyDown(window, { key: "z", ctrlKey: true });
    expect(mocks.commands.overlayUndo).toHaveBeenCalledOnce();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(mocks.commands.overlayFinish).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: /para concluir/ }));
    expect(mocks.commands.overlayFinish).toHaveBeenCalledTimes(2);
    fireEvent.keyDown(window, { key: "q" });
    fireEvent.pointerLeave(root);
    expect(screen.queryByText("30, 40")).toBeNull();
    await act(async () => {});
  });

  it("renders nothing until the session state arrives", () => {
    mocks.commands.overlayState.mockReturnValue(new Promise(() => {}));
    const { container } = render(<OverlayWindow />);
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

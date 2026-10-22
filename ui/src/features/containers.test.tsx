import { act, createEvent, fireEvent, render, screen, waitFor } from "@testing-library/react";
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
    overlayFocus: vi.fn(),
    overlayPickArea: vi.fn(),
    overlaySetMode: vi.fn(),
    overlayConfirmArea: vi.fn(),
    profileActivate: vi.fn(),
    profileCreate: vi.fn(),
    profileRename: vi.fn(),
    profileDuplicate: vi.fn(),
    profileExport: vi.fn(),
    profileDelete: vi.fn(),
    profileImport: vi.fn(),
    profileImportText: vi.fn(),
    hotkeySet: vi.fn(),
    settingsUpdate: vi.fn(),
    settingsAcknowledgeAntiCheat: vi.fn(),
    appOpenSupport: vi.fn(),
    engineRunBenchmark: vi.fn(),
    hudResize: vi.fn(),
    updateCheck: vi.fn(),
    updateInstall: vi.fn(),
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
      engineDegraded: event,
      settingsChanged: event,
      profilesChanged: event,
      capabilities: event,
      hotkeyStatusChanged: event,
      hotkeyActionFailed: event,
      hotkeyRecording: event,
      overlayChanged: event,
      navigateRequested: event,
      hudHover: event,
      updateAvailable: event,
    },
  };
});

const { MainWindow } = await import("./main/main-window");
const { VariationWindow } = await import("./variation/variation-window");
const { OverlayWindow } = await import("./overlay/overlay-window");
const { HudWindow } = await import("./hud/hud-window");
const { useAppStore } = await import("../store/app-store");

function bootstrap(profile: Profile = prototypeProfile()) {
  return {
    version: "0.1.0",
    dpiAwareness: "PerMonitorV2",
    settings: {
      activeProfileId: profile.id,
      locale: "pt-BR",
      theme: "grafite",
      followSystemTheme: false,
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
    fireEvent.click(screen.getByRole("button", { name: /^Atalhos/ }));
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

describe("Anti-cheat notice and settings", () => {
  it("asks once before the first start, then starts", async () => {
    mocks.commands.engineStart
      .mockResolvedValueOnce({ status: "error", error: { code: "ANTI_CHEAT_ACK", message: "" } })
      .mockResolvedValueOnce({ status: "ok", data: null });
    mocks.commands.settingsAcknowledgeAntiCheat.mockResolvedValue(bootstrap().settings);
    render(<MainWindow />);
    fireEvent.click(await screen.findByRole("button", { name: /Iniciar/ }));
    const dialog = await screen.findByRole("alertdialog");
    expect(dialog).toHaveTextContent("Jogos online com anti-cheat");
    fireEvent.click(screen.getByRole("button", { name: "Entendi, iniciar" }));
    await waitFor(() => expect(mocks.commands.engineStart).toHaveBeenCalledTimes(2));
    expect(mocks.commands.settingsAcknowledgeAntiCheat).toHaveBeenCalledOnce();
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("closes the notice without starting", async () => {
    mocks.commands.engineStart.mockResolvedValue({ status: "error", error: { code: "ANTI_CHEAT_ACK", message: "" } });
    render(<MainWindow />);
    fireEvent.click(await screen.findByRole("button", { name: /Iniciar/ }));
    await screen.findByRole("alertdialog");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("alertdialog")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /Iniciar/ }));
    await screen.findByRole("alertdialog");
    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(mocks.commands.settingsAcknowledgeAntiCheat).not.toHaveBeenCalled();
  });

  it("saves preferences, opens Ko-fi and measures again", async () => {
    mocks.commands.settingsUpdate
      .mockResolvedValueOnce({ status: "ok", data: null })
      .mockResolvedValueOnce({ status: "error", error: { code: "AUTOSTART_FAILED", message: "" } });
    mocks.commands.appOpenSupport.mockResolvedValue({ status: "error", error: { code: "OPEN_FAILED", message: "" } });
    mocks.commands.engineRunBenchmark
      .mockResolvedValueOnce({ status: "ok", data: { minReliableIntervalMs: 1, measured: true, results: [] } })
      .mockResolvedValueOnce({ status: "error", error: "busy" });
    render(<MainWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "Configurações" }));
    fireEvent.click(screen.getByRole("switch", { name: "Mostrar HUD ao rodar" }));
    expect(mocks.commands.settingsUpdate).toHaveBeenCalledWith(expect.objectContaining({ showHud: true }));
    fireEvent.click(screen.getByRole("radio", { name: "Español" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("inicialização com o Windows");
    fireEvent.click(screen.getByRole("button", { name: "Apoiar ↗" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("navegador");
    fireEvent.click(screen.getByRole("button", { name: /Avançado/ }));
    fireEvent.click(screen.getByRole("button", { name: "Medir novamente" }));
    expect(await screen.findByText("1 ms")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Medir novamente" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("medir o agendador");
  });
});

describe("Updates", () => {
  it("checks, finds a version, installs it and reports failures", async () => {
    mocks.commands.updateCheck
      .mockResolvedValueOnce({ status: "ok", data: null })
      .mockResolvedValueOnce({ status: "error", error: { code: "UPDATE_FAILED", message: "" } })
      .mockResolvedValueOnce({ status: "ok", data: { version: "0.1.1", notes: null } });
    mocks.commands.updateInstall.mockResolvedValue({ status: "error", error: { code: "UPDATE_FAILED", message: "" } });
    render(<MainWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "Configurações" }));
    fireEvent.click(screen.getByRole("button", { name: "Verificar atualizações" }));
    fireEvent.click(await screen.findByRole("button", { name: "Você está na versão mais recente" }));
    fireEvent.click(await screen.findByRole("button", { name: "Não foi possível verificar" }));
    fireEvent.click(await screen.findByRole("button", { name: "Instalar a 0.1.1 e reiniciar" }));
    expect(mocks.commands.updateInstall).toHaveBeenCalledOnce();
    expect(await screen.findByRole("alert")).toHaveTextContent("Não foi possível atualizar");
    fireEvent.click(screen.getByRole("button", { name: "Voltar" }));
    fireEvent.click(await screen.findByRole("button", { name: "Atualizar ›" }));
    expect(screen.getByRole("heading", { name: "Configurações" })).toBeInTheDocument();
  });
});

describe("HudWindow", () => {
  it("follows the engine and reports its size", async () => {
    mocks.commands.appBootstrap.mockResolvedValue({
      ...bootstrap(),
      engine: tick({ state: "running", clicks: 42, remainingMs: 60_000 }),
    });
    render(<HudWindow />);
    expect(await screen.findByRole("status")).toHaveTextContent("42");
    expect(screen.getByText("Segure Ctrl para mover")).toBeInTheDocument();
    await waitFor(() => expect(mocks.commands.hudResize).toHaveBeenCalled());
  });

  it("renders nothing before the state arrives", () => {
    mocks.commands.appBootstrap.mockReturnValue(new Promise(() => {}));
    const { container } = render(<HudWindow />);
    expect(container).toBeEmptyDOMElement();
  });
});

describe("ThemesRoute", () => {
  it("opens from Configurações, picks a theme and toggles Seguir o Windows", async () => {
    mocks.commands.settingsUpdate
      .mockResolvedValueOnce({ status: "ok", data: null })
      .mockResolvedValueOnce({ status: "error", error: { code: "SETTINGS_INVALID", message: "" } });
    render(<MainWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "Configurações" }));
    fireEvent.click(screen.getByRole("button", { name: /Grafite ›/ }));
    expect(screen.getByRole("heading", { name: "Tema" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Fósforo · Terminal" }));
    expect(mocks.commands.settingsUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ theme: "fosforo", followSystemTheme: false }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Seguir o Windows" }));
    expect(mocks.commands.settingsUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({ followSystemTheme: true }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent("SETTINGS_INVALID");
    fireEvent.click(screen.getByRole("button", { name: "Voltar" }));
    expect(screen.getByRole("heading", { name: "Configurações" })).toBeInTheDocument();
  });

  it("labels the Tema row when following Windows", async () => {
    mocks.commands.appBootstrap.mockResolvedValue({
      ...bootstrap(),
      settings: { ...bootstrap().settings, followSystemTheme: true },
    });
    render(<MainWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "Configurações" }));
    expect(screen.getByRole("button", { name: /Seguindo o Windows · (Grafite|Papel) ›/ })).toBeInTheDocument();
  });
});

describe("ProfilesRoute", () => {
  const ok = (data: unknown = null) => ({ status: "ok", data });

  async function openProfiles() {
    render(<MainWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "Perfis" }));
    expect(await screen.findByRole("heading", { name: "Perfis" })).toBeInTheDocument();
  }

  it("switches profiles from the header dropdown", async () => {
    const other = { ...prototypeProfile(), id: "other", name: "Outro" };
    mocks.commands.appBootstrap.mockResolvedValue({ ...bootstrap(), profiles: [prototypeProfile(), other] });
    mocks.commands.profileActivate.mockResolvedValue({
      status: "error",
      error: { code: "PROFILE_NOT_FOUND", message: "" },
    });
    render(<MainWindow />);
    fireEvent.click(await screen.findByRole("button", { name: /Coleta rápida/ }));
    fireEvent.click(screen.getByRole("menuitemradio", { name: "Outro" }));
    expect(mocks.commands.profileActivate).toHaveBeenCalledWith("other");
    expect(await screen.findByRole("alert")).toHaveTextContent("Perfil não encontrado.");
  });

  it("drives every command and reports results", async () => {
    mocks.commands.profileActivate.mockResolvedValue(ok());
    mocks.commands.profileCreate.mockResolvedValue(ok());
    mocks.commands.profileDuplicate.mockResolvedValue({
      status: "error",
      error: { code: "NAME_TOO_LONG", message: "" },
    });
    mocks.commands.profileExport.mockResolvedValueOnce(ok("C:\\x.chprofile")).mockResolvedValueOnce(ok(null));
    mocks.commands.profileRename.mockResolvedValue({ status: "error", error: { code: "NAME_TAKEN", message: "" } });
    await openProfiles();

    fireEvent.click(screen.getByRole("button", { name: "Coleta rápida" }));
    expect(mocks.commands.profileActivate).toHaveBeenCalledWith("proto");
    fireEvent.click(screen.getByRole("button", { name: "+ Novo" }));
    expect(mocks.commands.profileCreate).toHaveBeenCalledWith("Novo perfil");
    fireEvent.keyDown(screen.getByRole("button", { name: "Coleta rápida" }), { key: "d", ctrlKey: true });
    expect(await screen.findByRole("alert")).toHaveTextContent("até 48 caracteres");
    fireEvent.click(screen.getByRole("button", { name: "Exportar" }));
    expect(await screen.findByText("Exportado para C:\\x.chprofile")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Exportar" }));
    await act(async () => {});
    expect(screen.queryByText(/Exportado/)).toBeNull();

    fireEvent.keyDown(screen.getByRole("button", { name: "Coleta rápida" }), { key: "F2" });
    const input = screen.getByRole("textbox", { name: "Nome do perfil" });
    fireEvent.change(input, { target: { value: "Outro" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(await screen.findByText("Já existe um perfil com esse nome.")).toBeInTheDocument();
    fireEvent.keyDown(input, { key: "Escape" });

    fireEvent.click(screen.getByRole("button", { name: "Voltar" }));
    expect(await screen.findByText("Coleta rápida")).toBeInTheDocument();
  });

  it("explains imports, their notes and refused files", async () => {
    const profile = { ...prototypeProfile(), id: "new", name: "Coleta rápida (2)" };
    mocks.commands.profileImport
      .mockResolvedValueOnce(
        ok({
          profile,
          notes: [
            { kind: "renamed", from: "Coleta rápida", to: "Coleta rápida (2)" },
            { kind: "hotkeyDropped", combo: { mods: [], key: "F6" } },
            { kind: "migrated", fromVersion: 0 },
          ],
        }),
      )
      .mockResolvedValueOnce(ok(null))
      .mockResolvedValueOnce(ok({ profile, notes: [] }));
    mocks.commands.profileImportText.mockResolvedValue({
      status: "error",
      error: { code: "IMPORT_NOT_CHPROFILE", message: "" },
    });
    await openProfiles();

    fireEvent.click(screen.getByRole("button", { name: "Importar" }));
    expect(await screen.findByText("Perfil “Coleta rápida (2)” importado.")).toBeInTheDocument();
    expect(screen.getByText(/ficou “Coleta rápida \(2\)”/)).toBeInTheDocument();
    expect(screen.getByText(/O atalho F6 já estava em uso/)).toBeInTheDocument();
    expect(screen.getByText(/versão anterior/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Importar" }));
    await act(async () => {});
    expect(screen.queryByText(/importado/)).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Importar" }));
    expect(await screen.findByText("Perfil “Coleta rápida (2)” importado.")).toBeInTheDocument();

    const root = screen.getByRole("heading", { name: "Perfis" }).closest(".relative") as HTMLElement;
    const drop = (file: File) => {
      const event = createEvent.drop(root);
      Object.defineProperty(event, "dataTransfer", { value: { types: ["Files"], files: [file] } });
      fireEvent(root, event);
    };
    const big = new File(["x"], "big.chprofile");
    Object.defineProperty(big, "size", { value: 2 * 1024 * 1024 });
    drop(big);
    expect(await screen.findByRole("alert")).toHaveTextContent("grande demais");
    const small = new File(["{}"], "x.chprofile");
    Object.defineProperty(small, "text", { value: async () => "{}" });
    drop(small);
    await waitFor(() => expect(mocks.commands.profileImportText).toHaveBeenCalledWith("{}"));
    expect(await screen.findByRole("alert")).toHaveTextContent("não é um .chprofile");
  });

  it("records, clears and deletes through the shared shortcut recorder", async () => {
    mocks.commands.hotkeySet.mockResolvedValue(ok({ result: "saved", statuses: [] }));
    mocks.commands.profileDelete.mockResolvedValue(ok());
    const other = { ...prototypeProfile(), id: "other", name: "Outro", hotkey: { mods: ["ctrl"], key: "2" } };
    mocks.commands.appBootstrap.mockResolvedValue({ ...bootstrap(), profiles: [prototypeProfile(), other] });
    await openProfiles();
    fireEvent.click(screen.getByRole("button", { name: "Ações de Outro" }));
    fireEvent.click(screen.getByRole("menuitem", { name: /Remover atalho/ }));
    expect(mocks.commands.hotkeySet).toHaveBeenCalledWith({ kind: "profile", id: "other" }, null, false);
    fireEvent.click(screen.getByRole("button", { name: "Ações de Outro" }));
    fireEvent.click(screen.getByRole("menuitem", { name: /Definir atalho/ }));
    expect(mocks.commands.hotkeyBeginRecord).toHaveBeenCalledOnce();
    expect(screen.getByText("Pressione as teclas…")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Ações de Coleta rápida" }));
    fireEvent.click(screen.getByRole("menuitem", { name: /Excluir/ }));
    fireEvent.click(screen.getByRole("button", { name: "Excluir" }));
    expect(mocks.commands.profileDelete).toHaveBeenCalledWith("proto");
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

  it("opens the area overlay from the scan panel and explains a failure", async () => {
    const profile = { ...prototypeProfile(), mode: "scan" as const };
    mocks.commands.appBootstrap.mockResolvedValue(bootstrap(profile));
    mocks.commands.overlayPickArea.mockResolvedValue({
      status: "error",
      error: { code: "OVERLAY_OPEN", message: "open" },
    });
    render(<MainWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "Selecionar área" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("A captura de tela já está aberta.");
    mocks.commands.overlayPickArea.mockResolvedValue({ status: "ok", data: null });
    fireEvent.click(screen.getByRole("button", { name: "Selecionar área" }));
    await act(async () => {});
    expect(mocks.commands.overlayPickArea).toHaveBeenCalledTimes(2);
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
    expect(mocks.commands.overlayFocus).toHaveBeenCalledWith(0);
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

  it("starts the crosshair at the cursor so space and arrows work before the mouse moves", async () => {
    mocks.commands.overlayState.mockResolvedValue({ status: "ok", data: { ...state, cursor: { x: 50, y: 60 } } });
    mocks.commands.overlayAddPoint.mockResolvedValue({ status: "ok", data: state });
    render(<OverlayWindow />);
    expect(await screen.findByText("50, 60")).toBeInTheDocument();
    expect(mocks.commands.overlayFocus).toHaveBeenCalledWith(0);
    fireEvent.keyDown(window, { key: " " });
    expect(mocks.commands.overlayAddPoint).toHaveBeenLastCalledWith(0, 50, 60);
    fireEvent.keyDown(window, { key: "ArrowDown", shiftKey: true });
    expect(screen.getByText("50, 70")).toBeInTheDocument();
    await act(async () => {});
  });

  it("moves the area with the arrows and resizes it with ctrl", async () => {
    mocks.commands.overlayState.mockResolvedValue({
      status: "ok",
      data: { ...state, mode: "area", area: { x: 100, y: 100, w: 200, h: 100 } },
    });
    render(<OverlayWindow />);
    expect(await screen.findByText("200 × 100")).toBeInTheDocument();
    expect(screen.getByText("Setas movem · Ctrl+setas redimensionam")).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "ArrowRight" });
    fireEvent.keyDown(window, { key: "ArrowDown", shiftKey: true });
    expect(screen.getByText("X 101 · Y 110")).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "ArrowLeft", ctrlKey: true });
    fireEvent.keyDown(window, { key: "ArrowUp", ctrlKey: true, shiftKey: true });
    expect(screen.getByText("199 × 90")).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "q" });
    expect(screen.getByText("199 × 90")).toBeInTheDocument();
  });

  it("ignores the arrows in area mode before an area exists", async () => {
    mocks.commands.overlayState.mockResolvedValue({ status: "ok", data: { ...state, mode: "area", area: null } });
    render(<OverlayWindow />);
    expect(await screen.findByText("Arraste para selecionar")).toBeInTheDocument();
    const press = new KeyboardEvent("keydown", { key: "ArrowRight", cancelable: true });
    window.dispatchEvent(press);
    expect(press.defaultPrevented).toBe(false);
  });

  it("draws, moves and resizes an area, then confirms it", async () => {
    mocks.commands.overlayState.mockResolvedValue({
      status: "ok",
      data: { ...state, mode: "area", area: { x: 100, y: 100, w: 200, h: 100 } },
    });
    const view = render(<OverlayWindow />);
    expect(await screen.findByText("Arraste para selecionar")).toBeInTheDocument();
    expect(screen.getByText("200 × 100")).toBeInTheDocument();
    const root = view.container.firstElementChild as HTMLElement;

    fireEvent.pointerDown(root, { button: 0, clientX: 10, clientY: 20 });
    fireEvent.pointerMove(root, { clientX: 410, clientY: 220 });
    fireEvent.pointerUp(root);
    expect(screen.getByText("400 × 200")).toBeInTheDocument();
    fireEvent.pointerMove(root, { clientX: 900, clientY: 900 });
    expect(screen.getByText("400 × 200"), "moves without a drag do nothing").toBeInTheDocument();

    fireEvent.pointerDown(view.container.querySelector(".area-rect") as Element, { clientX: 100, clientY: 100 });
    fireEvent.pointerMove(root, { clientX: 150, clientY: 130 });
    fireEvent.pointerUp(root);
    expect(screen.getByText("X 60 · Y 50")).toBeInTheDocument();

    fireEvent.pointerDown(view.container.querySelector('[data-handle="se"]') as Element, {
      clientX: 460,
      clientY: 250,
    });
    fireEvent.pointerMove(root, { clientX: 500, clientY: 260 });
    fireEvent.pointerUp(root);
    expect(screen.getByText("440 × 210")).toBeInTheDocument();

    fireEvent.pointerDown(root, { button: 0, clientX: 800, clientY: 800 });
    fireEvent.pointerMove(root, { clientX: 801, clientY: 801 });
    fireEvent.pointerUp(root);
    expect(screen.getByText("440 × 210")).toBeInTheDocument();
    fireEvent.pointerDown(root, { button: 2 });
    fireEvent.keyDown(window, { key: "a" });
    fireEvent.keyDown(window, { key: "Enter" });
    expect(mocks.commands.overlayConfirmArea).toHaveBeenCalledWith(0, 60, 50, 440, 210);
    fireEvent.click(screen.getByRole("radio", { name: "Ponto" }));
    expect(mocks.commands.overlaySetMode).toHaveBeenCalledWith("points");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(mocks.commands.overlayFinish).toHaveBeenCalledOnce();
    await act(async () => {});
  });

  it("ignores Enter until a usable area exists", async () => {
    mocks.commands.overlayState.mockResolvedValue({ status: "ok", data: { ...state, mode: "area", area: null } });
    const view = render(<OverlayWindow />);
    await screen.findByText("Arraste para selecionar");
    fireEvent.keyDown(window, { key: "Enter" });
    const root = view.container.firstElementChild as HTMLElement;
    fireEvent.pointerDown(root, { button: 0, clientX: 10, clientY: 10 });
    fireEvent.pointerMove(root, { clientX: 15, clientY: 15 });
    fireEvent.keyDown(window, { key: "Enter" });
    expect(mocks.commands.overlayConfirmArea).not.toHaveBeenCalled();
    await act(async () => {});
  });

  it("does not pre-select an area that belongs to another monitor", async () => {
    mocks.commands.overlayState.mockResolvedValue({
      status: "ok",
      data: { ...state, mode: "area", area: { x: 5000, y: 0, w: 100, h: 100 } },
    });
    const view = render(<OverlayWindow />);
    await screen.findByText("Arraste para selecionar");
    expect(view.container.querySelector(".area-rect")).toBeNull();
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

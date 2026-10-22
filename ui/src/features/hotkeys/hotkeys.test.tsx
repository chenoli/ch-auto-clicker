import { act, fireEvent, render, renderHook, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import "../../i18n";

import type { HotkeyBindings, HotkeyStatus } from "../../ipc";

const mocks = vi.hoisted(() => ({
  commands: {
    hotkeySet: vi.fn(),
    hotkeyBeginRecord: vi.fn(),
    hotkeyRecordKey: vi.fn(),
    hotkeyCancelRecord: vi.fn(),
  },
  recordingListener: null as null | ((e: { payload: unknown }) => void),
}));

vi.mock("../../ipc", () => ({
  inTauri: true,
  commands: mocks.commands,
  events: {
    hotkeyRecording: {
      listen: vi.fn(async (cb: (e: { payload: unknown }) => void) => {
        mocks.recordingListener = cb;
        return () => (mocks.recordingListener = null);
      }),
    },
  },
}));

const { HotkeysScreen } = await import("./hotkeys-screen");
const { useHotkeyRecording } = await import("./use-hotkey-recording");
const { useAppStore } = await import("../../store/app-store");

const bindings: HotkeyBindings = {
  startStop: { mods: [], key: "F6" },
  pauseResume: { mods: [], key: "F7" },
  capturePoint: { mods: ["ctrl", "shift"], key: "P" },
  nextProfile: null,
  emergencyStop: { mods: ["ctrl", "alt"], key: "Escape" },
};

function screenProps(overrides = {}) {
  return {
    bindings,
    statuses: [] as HotkeyStatus[],
    recording: null,
    heldMods: [],
    conflict: null,
    onRecord: vi.fn(),
    onReplace: vi.fn(),
    onKeep: vi.fn(),
    onReset: vi.fn(),
    onRetry: vi.fn(),
    onBack: vi.fn(),
    onMinimize: vi.fn(),
    onClose: vi.fn(),
    ...overrides,
  };
}

const field = (name: RegExp) => screen.getByRole("button", { name });

beforeEach(() => useAppStore.setState({ profiles: [] }));
afterEach(() => vi.resetAllMocks());

describe("HotkeysScreen", () => {
  it("lists every action with its keys and the emergency note", () => {
    const props = screenProps();
    render(<HotkeysScreen {...props} />);
    expect(field(/Iniciar \/ Parar: F6/)).toHaveAttribute("data-state", "normal");
    expect(field(/Capturar ponto: Ctrl Shift P/)).toHaveTextContent("CtrlShiftP");
    expect(field(/Próximo perfil: —/)).toHaveTextContent("—");
    expect(screen.getByText(/Parada de emergência não pode ser desativada/)).toBeInTheDocument();
    fireEvent.click(field(/Pausar \/ Retomar/));
    expect(props.onRecord).toHaveBeenCalledWith("pauseResume");
    fireEvent.click(screen.getByRole("button", { name: "Restaurar padrão" }));
    fireEvent.click(screen.getByRole("button", { name: "Voltar" }));
    expect(props.onReset).toHaveBeenCalledOnce();
    expect(props.onBack).toHaveBeenCalledOnce();
  });

  it("shows the recording state, live modifiers and the conflict actions", () => {
    const props = screenProps({
      recording: "startStop",
      conflict: {
        target: { kind: "action", id: "capturePoint" },
        combo: { mods: [], key: "F7" },
        withName: "Pausar / Retomar",
      },
    });
    const { rerender } = render(<HotkeysScreen {...props} />);
    expect(field(/Iniciar \/ Parar/)).toHaveAttribute("data-state", "recording");
    expect(screen.getByText("Pressione as teclas…")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("F7 já está em uso por “Pausar / Retomar”.");
    fireEvent.click(screen.getByRole("button", { name: "Substituir" }));
    fireEvent.click(screen.getByRole("button", { name: "Manter o anterior" }));
    expect(props.onReplace).toHaveBeenCalledOnce();
    expect(props.onKeep).toHaveBeenCalledOnce();

    rerender(<HotkeysScreen {...props} heldMods={["ctrl", "alt"]} conflict={null} />);
    expect(field(/Iniciar \/ Parar/)).toHaveTextContent("CtrlAlt");
  });

  it("explains a shortcut taken by another app and retries", () => {
    const props = screenProps({
      statuses: [
        {
          target: { kind: "action", id: "startStop" },
          combo: { mods: [], key: "F6" },
          ok: false,
          errorCode: "HOTKEY_TAKEN",
        },
      ],
    });
    render(<HotkeysScreen {...props} />);
    expect(field(/Iniciar \/ Parar/)).toHaveAttribute("data-state", "conflict");
    expect(screen.getByRole("alert")).toHaveTextContent("Outro aplicativo já usa este atalho");
    fireEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));
    expect(props.onRetry).toHaveBeenCalledOnce();
  });
});

describe("useHotkeyRecording", () => {
  const emit = (payload: unknown) => act(() => mocks.recordingListener?.({ payload }));

  it("records a combo and saves it", async () => {
    mocks.commands.hotkeySet.mockResolvedValue({ status: "ok", data: { result: "saved", statuses: [] } });
    const { result } = renderHook(() => useHotkeyRecording());
    act(() => result.current.record("startStop"));
    expect(mocks.commands.hotkeyBeginRecord).toHaveBeenCalledOnce();
    await vi.waitFor(() => expect(mocks.recordingListener).not.toBeNull());
    emit({ mods: ["ctrl"], combo: null, cancelled: false });
    expect(result.current.heldMods).toEqual(["ctrl"]);
    emit({ mods: ["ctrl"], combo: { mods: ["ctrl"], key: "F9" }, cancelled: false });
    expect(result.current.recording).toBeNull();
    await vi.waitFor(() =>
      expect(mocks.commands.hotkeySet).toHaveBeenCalledWith(
        { kind: "action", id: "startStop" },
        { mods: ["ctrl"], key: "F9" },
        false,
      ),
    );
  });

  it("feeds keys pressed in the focused window to the recorder and keeps them from the page", async () => {
    const { result, unmount } = renderHook(() => useHotkeyRecording());
    const page = vi.fn();
    window.addEventListener("keydown", page);
    act(() => result.current.record("startStop"));
    const down = new KeyboardEvent("keydown", { code: "F5", cancelable: true, bubbles: true });
    window.dispatchEvent(down);
    expect(down.defaultPrevented).toBe(true);
    expect(page).not.toHaveBeenCalled();
    window.dispatchEvent(new KeyboardEvent("keydown", { code: "F5", repeat: true }));
    window.dispatchEvent(new KeyboardEvent("keyup", { code: "F5" }));
    window.dispatchEvent(new KeyboardEvent("keydown", { code: "IntlRo" }));
    expect(mocks.commands.hotkeyRecordKey.mock.calls).toEqual([
      ["F5", true],
      ["F5", false],
    ]);
    const side = new MouseEvent("mousedown", { button: 3, cancelable: true });
    window.dispatchEvent(side);
    window.dispatchEvent(new MouseEvent("mouseup", { button: 3 }));
    window.dispatchEvent(new MouseEvent("mousedown", { button: 0 }));
    expect(side.defaultPrevented).toBe(true);
    expect(mocks.commands.hotkeyRecordKey).toHaveBeenLastCalledWith("MouseX1", true);
    expect(mocks.commands.hotkeyRecordKey).toHaveBeenCalledTimes(3);
    unmount();
    window.removeEventListener("keydown", page);
  });

  it("stops listening to the window when recording ends", async () => {
    const { result } = renderHook(() => useHotkeyRecording());
    act(() => result.current.record("startStop"));
    act(() => result.current.cancel());
    const down = new KeyboardEvent("keydown", { code: "F8", cancelable: true });
    window.dispatchEvent(down);
    expect(down.defaultPrevented).toBe(false);
    expect(mocks.commands.hotkeyRecordKey).not.toHaveBeenCalled();
  });

  it("turns a conflict into a pending choice and replaces on request", async () => {
    mocks.commands.hotkeySet
      .mockResolvedValueOnce({
        status: "ok",
        data: { result: "conflict", with: { kind: "action", id: "pauseResume" } },
      })
      .mockResolvedValueOnce({ status: "ok", data: { result: "saved", statuses: [] } });
    const { result } = renderHook(() => useHotkeyRecording());
    act(() => result.current.record("startStop"));
    await vi.waitFor(() => expect(mocks.recordingListener).not.toBeNull());
    emit({ mods: [], combo: { mods: [], key: "F7" }, cancelled: false });
    await vi.waitFor(() =>
      expect(result.current.conflict).toEqual({
        target: { kind: "action", id: "startStop" },
        combo: { mods: [], key: "F7" },
        withName: "Pausar / Retomar",
      }),
    );
    act(() => result.current.replace());
    await vi.waitFor(() => expect(result.current.conflict).toBeNull());
    expect(mocks.commands.hotkeySet).toHaveBeenLastCalledWith(
      { kind: "action", id: "startStop" },
      { mods: [], key: "F7" },
      true,
    );
  });

  it("names profile conflicts and keeps the previous binding", async () => {
    useAppStore.setState({ profiles: [{ id: "p1", name: "Farm" }] as never });
    mocks.commands.hotkeySet.mockResolvedValue({
      status: "ok",
      data: { result: "conflict", with: { kind: "profile", id: "p1" } },
    });
    const { result } = renderHook(() => useHotkeyRecording());
    act(() => result.current.record("nextProfile"));
    await vi.waitFor(() => expect(mocks.recordingListener).not.toBeNull());
    emit({ mods: [], combo: { mods: [], key: "F9" }, cancelled: false });
    await vi.waitFor(() => expect(result.current.conflict?.withName).toBe("Atalho do perfil Farm"));
    act(() => result.current.keep());
    expect(result.current.conflict).toBeNull();
  });

  it("reports refused combos and handles cancellation", async () => {
    mocks.commands.hotkeySet.mockResolvedValue({ status: "error", error: { code: "HOTKEY_UNKNOWN_KEY", message: "" } });
    const { result } = renderHook(() => useHotkeyRecording());
    act(() => result.current.record("startStop"));
    await vi.waitFor(() => expect(mocks.recordingListener).not.toBeNull());
    emit({ mods: [], combo: null, cancelled: true });
    expect(result.current.recording).toBeNull();

    act(() => result.current.record("startStop"));
    await vi.waitFor(() => expect(mocks.recordingListener).not.toBeNull());
    emit({ mods: [], combo: { mods: [], key: "Pause" }, cancelled: false });
    await vi.waitFor(() => expect(useAppStore.getState().commandError).toBe("HOTKEY_UNKNOWN_KEY"));

    act(() => result.current.record("startStop"));
    act(() => result.current.cancel());
    expect(mocks.commands.hotkeyCancelRecord).toHaveBeenCalledOnce();
    act(() => result.current.cancel());
    expect(mocks.commands.hotkeyCancelRecord).toHaveBeenCalledOnce();
    act(() => result.current.replace());
  });
});

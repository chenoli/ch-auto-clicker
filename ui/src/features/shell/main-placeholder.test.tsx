import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import "../../i18n";

const ipc = vi.hoisted(() => ({
  inTauri: true,
  appBootstrap: vi.fn(),
  debugClickOnce: vi.fn(),
}));

vi.mock("../../ipc", () => ({
  get inTauri() {
    return ipc.inTauri;
  },
  commands: { appBootstrap: ipc.appBootstrap, debugClickOnce: ipc.debugClickOnce },
}));

const { MainPlaceholder } = await import("./main-placeholder");

describe("MainPlaceholder", () => {
  afterEach(() => {
    vi.resetAllMocks();
    ipc.inTauri = true;
  });

  it("shows the version and DPI mode reported by the backend", async () => {
    ipc.appBootstrap.mockResolvedValue({ version: "0.1.0", dpiAwareness: "PerMonitorV2" });
    render(<MainPlaceholder />);
    expect(await screen.findByText("v0.1.0 · PerMonitorV2")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Estrutura pronta." })).toBeInTheDocument();
  });

  it("schedules a dev test click and confirms it", async () => {
    ipc.appBootstrap.mockResolvedValue({ version: "0.1.0", dpiAwareness: "PerMonitorV2" });
    ipc.debugClickOnce.mockResolvedValue({ status: "ok", data: null });
    render(<MainPlaceholder />);
    fireEvent.click(screen.getByRole("button", { name: "Clique de teste em 2 s" }));
    expect(await screen.findByRole("button", { name: "Clique agendado. Posicione o cursor." })).toBeInTheDocument();
    expect(ipc.debugClickOnce).toHaveBeenCalledOnce();
  });

  it("keeps the label when the backend refuses the test click", async () => {
    ipc.appBootstrap.mockResolvedValue({ version: "0.1.0", dpiAwareness: "PerMonitorV2" });
    ipc.debugClickOnce.mockResolvedValue({ status: "error", error: "dev only" });
    render(<MainPlaceholder />);
    fireEvent.click(screen.getByRole("button", { name: "Clique de teste em 2 s" }));
    await waitFor(() => expect(ipc.debugClickOnce).toHaveBeenCalledOnce());
    expect(screen.getByRole("button", { name: "Clique de teste em 2 s" })).toBeInTheDocument();
  });

  it("does not call the backend outside Tauri", () => {
    ipc.inTauri = false;
    render(<MainPlaceholder />);
    expect(ipc.appBootstrap).not.toHaveBeenCalled();
    expect(screen.queryByText(/PerMonitorV2/)).not.toBeInTheDocument();
  });
});

import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import "../../i18n";

const ipc = vi.hoisted(() => ({
  inTauri: true,
  appBootstrap: vi.fn(),
}));

vi.mock("../../ipc", () => ({
  get inTauri() {
    return ipc.inTauri;
  },
  commands: { appBootstrap: ipc.appBootstrap },
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

  it("does not call the backend outside Tauri", () => {
    ipc.inTauri = false;
    render(<MainPlaceholder />);
    expect(ipc.appBootstrap).not.toHaveBeenCalled();
    expect(screen.queryByText(/PerMonitorV2/)).not.toBeInTheDocument();
  });
});

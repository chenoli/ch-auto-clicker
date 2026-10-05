import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import "../../i18n";

import { applyTheme, initialTheme, isThemeId, resolveTheme, syncTheme } from "../../styles/theme";

import { ThemesScreen } from "./themes-screen";

function mockSystem(dark: boolean) {
  const listeners = new Set<() => void>();
  const media = {
    matches: dark,
    addEventListener: (_: string, fn: () => void) => listeners.add(fn),
    removeEventListener: (_: string, fn: () => void) => listeners.delete(fn),
  };
  vi.stubGlobal("matchMedia", () => media);
  return {
    flip(next: boolean) {
      media.matches = next;
      listeners.forEach((fn) => fn());
    },
    listeners,
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

describe("theme helpers", () => {
  it("resolves Seguir o Windows to Grafite or Papel", () => {
    expect(resolveTheme({ theme: "blueprint", followSystemTheme: false }, true)).toBe("blueprint");
    expect(resolveTheme({ theme: "blueprint", followSystemTheme: true }, true)).toBe("grafite");
    expect(resolveTheme({ theme: "blueprint", followSystemTheme: true }, false)).toBe("papel");
    expect(isThemeId("fosforo")).toBe(true);
    expect(isThemeId("neon")).toBe(false);
  });

  it("remembers the last theme for the next window", () => {
    expect(initialTheme()).toBe("grafite");
    applyTheme("papel");
    expect(document.documentElement.dataset.theme).toBe("papel");
    expect(initialTheme()).toBe("papel");
    localStorage.setItem("ch.theme", "neon");
    expect(initialTheme()).toBe("grafite");
  });

  it("survives unavailable storage", () => {
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    applyTheme("bancada");
    expect(document.documentElement.dataset.theme).toBe("bancada");
    expect(initialTheme()).toBe("grafite");
    vi.restoreAllMocks();
  });

  it("follows settings and the Windows switch until disposed", () => {
    const system = mockSystem(true);
    let push: (c: { theme: "fosforo"; followSystemTheme: boolean }) => void = () => {};
    const unsubscribe = vi.fn();
    const dispose = syncTheme((apply) => {
      push = apply;
      return unsubscribe;
    });
    push({ theme: "fosforo", followSystemTheme: false });
    expect(document.documentElement.dataset.theme).toBe("fosforo");
    push({ theme: "fosforo", followSystemTheme: true });
    expect(document.documentElement.dataset.theme).toBe("grafite");
    system.flip(false);
    expect(document.documentElement.dataset.theme).toBe("papel");
    dispose();
    expect(unsubscribe).toHaveBeenCalledOnce();
    expect(system.listeners.size).toBe(0);
  });

  it("applies an initial choice right away", () => {
    mockSystem(false);
    const dispose = syncTheme(() => () => {}, { theme: "contraste", followSystemTheme: false });
    expect(document.documentElement.dataset.theme).toBe("contraste");
    dispose();
  });
});

describe("ThemesScreen", () => {
  it("shows the six themes in their own colors and selects one", () => {
    const props = {
      onSelect: vi.fn(),
      onToggleFollow: vi.fn(),
      onBack: vi.fn(),
      onMinimize: vi.fn(),
      onClose: vi.fn(),
    };
    const view = render(<ThemesScreen shown="papel" followSystem={false} {...props} />);
    expect(view.container.querySelectorAll(".theme-card")).toHaveLength(6);
    expect(view.container.querySelector('[data-theme="blueprint"]')).not.toBeNull();
    expect(screen.getByRole("button", { name: "Papel · Claro" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("05 · Claro")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Fósforo · Terminal" }));
    expect(props.onSelect).toHaveBeenCalledWith("fosforo");
    const follow = screen.getByRole("button", { name: "Seguir o Windows" });
    expect(follow).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(follow);
    expect(props.onToggleFollow).toHaveBeenCalledOnce();
    act(() => fireEvent.click(screen.getByRole("button", { name: "Voltar" })));
    expect(props.onBack).toHaveBeenCalledOnce();
  });
});

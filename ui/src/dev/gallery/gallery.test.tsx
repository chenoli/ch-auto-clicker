import { act, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import "../../i18n";

import { Gallery, parseGalleryHash } from "./gallery";

describe("parseGalleryHash", () => {
  it("reads screen and theme", () => {
    expect(parseGalleryHash("#/tokens?theme=papel")).toEqual({ screen: "tokens", theme: "papel", lang: "pt-BR" });
    expect(parseGalleryHash("#/tokens?lang=es-419").lang).toBe("es-419");
  });

  it("falls back to the default theme and the index", () => {
    expect(parseGalleryHash("")).toEqual({ screen: null, theme: "grafite", lang: "pt-BR" });
    expect(parseGalleryHash("#/tokens?theme=neon&lang=xx")).toEqual({
      screen: "tokens",
      theme: "grafite",
      lang: "pt-BR",
    });
  });
});

describe("Gallery", () => {
  afterEach(() => {
    window.location.hash = "";
  });

  // Renders every gallery screen at once, so it gets more time than the default under coverage.
  it("lists every screen on the index", { timeout: 20_000 }, () => {
    render(<Gallery />);
    expect(screen.getByRole("heading", { name: "Galeria de telas" })).toBeInTheDocument();
    expect(screen.getByTestId("screen-tokens")).toBeInTheDocument();
  });

  it("isolates one screen and applies the theme from the hash", () => {
    window.location.hash = "#/tokens?theme=papel";
    render(<Gallery />);
    expect(screen.queryByRole("heading", { name: "Galeria de telas" })).not.toBeInTheDocument();
    expect(document.documentElement.dataset.theme).toBe("papel");
  });

  it("follows hash changes", () => {
    render(<Gallery />);
    act(() => {
      window.location.hash = "#/tokens?theme=contraste";
      window.dispatchEvent(new HashChangeEvent("hashchange"));
    });
    expect(document.documentElement.dataset.theme).toBe("contraste");
  });
});

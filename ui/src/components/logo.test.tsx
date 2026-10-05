import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Logo } from "./logo";

describe("Logo", () => {
  it("renders the CH monogram with an accent dot", () => {
    render(<Logo />);
    const dot = screen.getByText(".");
    expect(dot).toHaveClass("text-accent");
    expect(dot.parentElement).toHaveTextContent("CH.");
    expect(dot.parentElement).toHaveClass("text-14");
  });

  it("supports the display size", () => {
    render(<Logo size="display" />);
    expect(screen.getByText(".").parentElement).toHaveClass("text-28");
  });
});

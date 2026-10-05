import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Badge } from "./badge";
import { BigNumberField } from "./big-number-field";
import { Button } from "./button";
import { Callout } from "./callout";
import { Kbd } from "./kbd";
import { RadioCard } from "./radio-card";
import { Segmented } from "./segmented";
import { Stepper } from "./stepper";
import { Toggle } from "./toggle";

describe("Segmented", () => {
  const options = [
    { value: "left", label: "Esquerdo" },
    { value: "right", label: "Direito" },
  ] as const;

  it("marks the active option and reports changes", () => {
    const onChange = vi.fn();
    render(<Segmented label="Botão" options={options} value="left" onChange={onChange} />);
    expect(screen.getByRole("radiogroup", { name: "Botão" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Esquerdo" })).toHaveAttribute("aria-checked", "true");
    fireEvent.click(screen.getByRole("radio", { name: "Direito" }));
    expect(onChange).toHaveBeenCalledWith("right");
  });

  it("renders indexes in the tab variant", () => {
    render(
      <Segmented
        label="Modo"
        variant="tabs"
        options={[{ value: "click", label: "Click", index: "01" }]}
        value="click"
        onChange={() => {}}
      />,
    );
    expect(screen.getByText("01")).toHaveClass("mode-tabs__index");
    expect(screen.getByRole("radiogroup")).toHaveClass("mode-tabs");
  });

  it("supports the compact variant", () => {
    render(<Segmented label="x" variant="compact" options={options} value="left" onChange={() => {}} />);
    expect(screen.getByRole("radiogroup")).toHaveClass("segmented--compact");
  });
});

describe("Toggle", () => {
  it("is a switch that flips its state", () => {
    const onChange = vi.fn();
    render(<Toggle label="Parar após" checked={false} onChange={onChange} />);
    const toggle = screen.getByRole("switch", { name: "Parar após" });
    expect(toggle).toHaveAttribute("aria-checked", "false");
    fireEvent.click(toggle);
    expect(onChange).toHaveBeenCalledWith(true);
  });
});

describe("RadioCard", () => {
  it("selects by click and keyboard, with trailing content", () => {
    const onSelect = vi.fn();
    render(<RadioCard label="Pontos na tela" checked={false} onSelect={onSelect} trailing={<span>4 pontos ›</span>} />);
    const card = screen.getByRole("radio", { name: /Pontos na tela/ });
    fireEvent.click(card);
    fireEvent.keyDown(card, { key: " " });
    fireEvent.keyDown(card, { key: "Enter" });
    fireEvent.keyDown(card, { key: "a" });
    expect(onSelect).toHaveBeenCalledTimes(3);
    expect(screen.getByText("4 pontos ›")).toBeInTheDocument();
  });
});

describe("Stepper", () => {
  it("steps and respects its limits", () => {
    const dec = vi.fn();
    const inc = vi.fn();
    render(
      <Stepper
        value="00:05:00"
        onDecrement={dec}
        onIncrement={inc}
        decrementLabel="Diminuir"
        incrementLabel="Aumentar"
        canDecrement={false}
        muted
      />,
    );
    expect(screen.getByText("00:05:00")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Diminuir" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Aumentar" }));
    expect(inc).toHaveBeenCalledOnce();
    expect(dec).not.toHaveBeenCalled();
    expect(screen.getByText("00:05:00").parentElement).toHaveAttribute("data-muted", "true");
  });
});

describe("BigNumberField", () => {
  it("reports raw input and commits on blur and Enter", () => {
    const onChange = vi.fn();
    const onCommit = vi.fn();
    render(<BigNumberField label="Intervalo" unit="ms" value="100" onChange={onChange} onCommit={onCommit} />);
    const input = screen.getByRole("textbox", { name: "Intervalo" });
    fireEvent.change(input, { target: { value: "250" } });
    expect(onChange).toHaveBeenCalledWith("250");
    fireEvent.keyDown(input, { key: "Enter" });
    fireEvent.keyDown(input, { key: "a" });
    fireEvent.blur(input);
    expect(onCommit).toHaveBeenCalledTimes(2);
    expect(screen.getByText("ms")).toBeInTheDocument();
  });
});

describe("small primitives", () => {
  it("renders keys, badges, buttons and callouts", () => {
    const onClick = vi.fn();
    render(
      <div>
        <Kbd size="large" broken>
          F6
        </Kbd>
        <Kbd>Esc</Kbd>
        <Badge>4 pontos</Badge>
        <Badge tone="neutral">Click</Badge>
        <Button variant="primary" onClick={onClick}>
          Aplicar
        </Button>
        <Button>Cancelar</Button>
        <Callout>Atenção</Callout>
        <Callout tone="danger">Erro</Callout>
      </div>,
    );
    expect(screen.getByText("F6")).toHaveClass("kbd--large");
    expect(screen.getByText("F6")).toHaveAttribute("data-broken", "true");
    expect(screen.getByText("Click")).toHaveClass("badge--neutral");
    fireEvent.click(screen.getByRole("button", { name: "Aplicar" }));
    expect(onClick).toHaveBeenCalledOnce();
    expect(screen.getByRole("button", { name: "Cancelar" })).toHaveClass("button--secondary");
    expect(screen.getByRole("note")).toHaveTextContent("Atenção");
    expect(screen.getByRole("alert")).toHaveTextContent("Erro");
  });
});

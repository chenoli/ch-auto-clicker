import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import "../../i18n";

import { VariationEditorView } from "./variation-editor-view";

import type { VariationEditorViewProps } from "./variation-editor-view";

function setup(overrides: Partial<VariationEditorViewProps> = {}) {
  const props: VariationEditorViewProps = {
    initialBlocks: [1, 35, 70],
    double: false,
    locale: "pt-BR",
    minReliableMs: 4,
    onApply: vi.fn(),
    onDisable: vi.fn(),
    onClose: vi.fn(),
    onMinimize: vi.fn(),
    ...overrides,
  };
  render(<VariationEditorView {...props} />);
  return props;
}

const block = (id: string) => screen.getByRole("option", { name: new RegExp(id) });
const field = () => screen.getByRole("textbox", { name: /Bloco/ });

describe("VariationEditorView", () => {
  it("shows the stats and the low-interval warning", () => {
    setup();
    expect(screen.getByText("Variação de intervalo")).toBeInTheDocument();
    expect(screen.getByText("106")).toBeInTheDocument();
    expect(screen.getByText("35,3")).toBeInTheDocument();
    expect(screen.getByText("28,3")).toBeInTheDocument();
    expect(screen.getByRole("note")).toHaveTextContent("Intervalos abaixo de 4 ms");
    expect(block("B01")).toHaveAttribute("data-low", "true");
  });

  it("hides the warning when every block is reliable", () => {
    setup({ minReliableMs: 1 });
    expect(screen.queryByRole("note")).not.toBeInTheDocument();
  });

  it("edits, adds, moves and removes blocks, then applies", () => {
    const props = setup();
    fireEvent.click(block("B02"));
    fireEvent.change(field(), { target: { value: "40x" } });
    fireEvent.keyDown(field(), { key: "Enter" });
    fireEvent.click(screen.getByRole("button", { name: "Adicionar bloco" }));
    fireEvent.click(screen.getByRole("button", { name: "← Mover" }));
    fireEvent.click(screen.getByRole("button", { name: "Mover →" }));
    fireEvent.click(block("B01"));
    fireEvent.click(screen.getByRole("button", { name: "Remover" }));
    fireEvent.click(screen.getByRole("button", { name: "Aplicar ao perfil" }));
    expect(props.onApply).toHaveBeenCalledWith([40, 70, 70]);
  });

  it("restores the value when the field is cleared and clamps big values", () => {
    const props = setup({ initialBlocks: [10] });
    fireEvent.change(field(), { target: { value: "" } });
    fireEvent.blur(field());
    expect(field()).toHaveValue("10");
    fireEvent.change(field(), { target: { value: "999999" } });
    fireEvent.blur(field());
    fireEvent.click(screen.getByRole("button", { name: "Aplicar ao perfil" }));
    expect(props.onApply).toHaveBeenCalledWith([60_000]);
    expect(screen.getByRole("button", { name: "Remover" })).toBeDisabled();
  });

  it("starts from 100 ms when there are no blocks and wires the other actions", () => {
    const props = setup({ initialBlocks: [], error: "STALE_PROFILE", busy: false });
    expect(block("B01")).toHaveTextContent("100");
    expect(screen.getByRole("alert")).toHaveTextContent("alterado em outra janela");
    fireEvent.click(screen.getByRole("button", { name: "Desativar variação" }));
    fireEvent.click(screen.getByRole("button", { name: "Fechar" }));
    fireEvent.click(screen.getByRole("button", { name: "Voltar" }));
    fireEvent.click(screen.getByRole("button", { name: "Minimizar" }));
    expect(props.onDisable).toHaveBeenCalledOnce();
    expect(props.onClose).toHaveBeenCalledTimes(2);
    expect(props.onMinimize).toHaveBeenCalledOnce();
  });

  it("stops adding at 32 blocks", () => {
    setup({ initialBlocks: Array.from({ length: 32 }, () => 10) });
    expect(screen.getByRole("button", { name: "Adicionar bloco" })).toBeDisabled();
  });
});

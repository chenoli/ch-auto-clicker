import { act, createEvent, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { prototypeProfile } from "../../dev/gallery/fixtures";

import i18n from "../../i18n";

import { Header } from "../main/header";
import { matchesSearch, profileSummary } from "./profile-summary";
import { ProfilesScreen } from "./profiles-screen";

import type { Profile } from "../../ipc";
import type { ProfilesScreenProps } from "./profiles-screen";

const t = i18n.t.bind(i18n);

function profiles(): Profile[] {
  const base = prototypeProfile();
  return [
    { ...base, id: "a", name: "Coleta rápida", hotkey: { mods: ["ctrl"], key: "1" } },
    { ...base, id: "b", name: "Leitura em loop", mode: "scan" },
    { ...base, id: "c", name: "Anti-ociosidade", mode: "wiggle" },
  ];
}

describe("profile summary and search", () => {
  it("summarizes each mode like the handoff cards", () => {
    const [click, scan, wiggle] = profiles() as [Profile, Profile, Profile];
    expect(profileSummary(click, "pt-BR", t)).toBe("Variação 1·35·70 ms · esquerdo · 4 pontos");
    const plain = {
      ...click,
      click: { ...click.click, kind: "double" as const, variation: { ...click.click.variation, enabled: false } },
    };
    plain.modes = { ...plain.modes, click: { ...plain.modes.click, position: "cursor" } };
    expect(profileSummary(plain, "pt-BR", t)).toBe("100 ms · esquerdo · duplo");
    expect(profileSummary(scan, "pt-BR", t)).toBe("sem área · 24 px · loop");
    const area = {
      ...scan,
      modes: { ...scan.modes, scan: { ...scan.modes.scan, area: { x: 0, y: 0, w: 776, h: 260 }, loop: false } },
    };
    expect(profileSummary(area, "pt-BR", t)).toBe("776×260 · 24 px · uma vez");
    expect(profileSummary(wiggle, "pt-BR", t)).toBe("0,05% · 8 Hz");
    const clicking = {
      ...wiggle,
      modes: { ...wiggle.modes, wiggle: { ...wiggle.modes.wiggle, clickWhileWiggling: true } },
    };
    expect(profileSummary(clicking, "pt-BR", t)).toBe("0,05% · 8 Hz · clica");
  });

  it("searches ignoring case and accents", () => {
    expect(matchesSearch("Coleta rápida", "RAPIDA")).toBe(true);
    expect(matchesSearch("Coleta rápida", "  col ")).toBe(true);
    expect(matchesSearch("Coleta rápida", "farm")).toBe(false);
  });
});

function setup(overrides: Partial<ProfilesScreenProps> = {}) {
  const props: ProfilesScreenProps = {
    profiles: profiles(),
    activeId: "a",
    locale: "pt-BR",
    dataPath: "%APPDATA%\\Chenoli\\CH Auto Clicker",
    recordingProfile: null,
    heldMods: [],
    conflict: null,
    onBack: vi.fn(),
    onMinimize: vi.fn(),
    onClose: vi.fn(),
    onActivate: vi.fn(),
    onCreate: vi.fn(),
    onRename: vi.fn(async () => null),
    onDuplicate: vi.fn(),
    onExport: vi.fn(),
    onDelete: vi.fn(),
    onImport: vi.fn(),
    onImportFile: vi.fn(),
    onRecordHotkey: vi.fn(),
    onClearHotkey: vi.fn(),
    onCancelRecord: vi.fn(),
    onReplaceHotkey: vi.fn(),
    onKeepHotkey: vi.fn(),
    ...overrides,
  };
  const view = render(<ProfilesScreen {...props} />);
  return { props, view };
}

const card = (name: string) => screen.getByRole("button", { name });
const openMenu = (name: string) => fireEvent.click(screen.getByRole("button", { name: `Ações de ${name}` }));

describe("ProfilesScreen", () => {
  it("lists the profiles like Tela 05.A and filters by name", () => {
    setup();
    expect(card("Coleta rápida")).toHaveAttribute("data-active", "true");
    expect(screen.getByText("Ctrl 1")).toBeInTheDocument();
    expect(screen.getByText("3 perfis · %APPDATA%\\Chenoli\\CH Auto Clicker")).toBeInTheDocument();
    fireEvent.change(screen.getByRole("searchbox", { name: "Buscar perfil" }), { target: { value: "loop" } });
    expect(screen.queryByText("Coleta rápida")).toBeNull();
    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "zzz" } });
    expect(screen.getByText("Nenhum perfil com esse nome.")).toBeInTheDocument();
  });

  it("activates, creates, imports and exports", () => {
    const { props } = setup();
    fireEvent.click(card("Leitura em loop"));
    expect(props.onActivate).toHaveBeenCalledWith("b");
    fireEvent.keyDown(card("Anti-ociosidade"), { key: "Enter" });
    expect(props.onActivate).toHaveBeenCalledWith("c");
    fireEvent.click(screen.getByRole("button", { name: "+ Novo" }));
    fireEvent.click(screen.getByRole("button", { name: "Importar" }));
    fireEvent.click(screen.getByRole("button", { name: "Exportar" }));
    expect(props.onCreate).toHaveBeenCalledOnce();
    expect(props.onImport).toHaveBeenCalledOnce();
    expect(props.onExport).toHaveBeenCalledWith("a");
    fireEvent.click(screen.getByRole("button", { name: "Voltar" }));
    expect(props.onBack).toHaveBeenCalledOnce();
  });

  it("runs every menu action and closes the menu on outside clicks", async () => {
    const { props } = setup();
    openMenu("Coleta rápida");
    const menu = screen.getByRole("menu");
    expect(within(menu).getByRole("menuitem", { name: /Remover atalho/ })).toBeInTheDocument();
    fireEvent.click(within(menu).getByRole("menuitem", { name: /Duplicar/ }));
    expect(props.onDuplicate).toHaveBeenCalledWith("a");
    expect(screen.queryByRole("menu")).toBeNull();

    openMenu("Coleta rápida");
    fireEvent.click(screen.getByRole("menuitem", { name: /Exportar…/ }));
    expect(props.onExport).toHaveBeenCalledWith("a");
    openMenu("Coleta rápida");
    fireEvent.click(screen.getByRole("menuitem", { name: /Definir atalho/ }));
    expect(props.onRecordHotkey).toHaveBeenCalledWith("a");
    openMenu("Coleta rápida");
    fireEvent.click(screen.getByRole("menuitem", { name: /Remover atalho/ }));
    expect(props.onClearHotkey).toHaveBeenCalledWith("a");

    openMenu("Leitura em loop");
    expect(screen.queryByRole("menuitem", { name: /Remover atalho/ })).toBeNull();
    await act(async () => new Promise((r) => setTimeout(r, 1)));
    fireEvent.click(document.body);
    expect(screen.queryByRole("menu")).toBeNull();
    openMenu("Leitura em loop");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("renames inline and shows a refused name", async () => {
    const onRename = vi.fn(async (_id: string, name: string) => (name === "Anti-ociosidade" ? "NAME_TAKEN" : null));
    setup({ onRename });
    openMenu("Leitura em loop");
    fireEvent.click(screen.getByRole("menuitem", { name: /Renomear/ }));
    const input = screen.getByRole("textbox", { name: "Nome do perfil" });
    fireEvent.change(input, { target: { value: "Anti-ociosidade" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(await screen.findByText("Já existe um perfil com esse nome.")).toBeInTheDocument();
    fireEvent.change(input, { target: { value: "Leitura" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(screen.queryByRole("textbox", { name: "Nome do perfil" })).toBeNull());
    expect(onRename).toHaveBeenLastCalledWith("b", "Leitura");

    fireEvent.keyDown(card("Coleta rápida"), { key: "F2" });
    const again = screen.getByRole("textbox", { name: "Nome do perfil" });
    fireEvent.keyDown(again, { key: "Escape" });
    expect(screen.queryByRole("textbox")).toBeNull();
    fireEvent.keyDown(card("Coleta rápida"), { key: "F2" });
    fireEvent.blur(screen.getByRole("textbox", { name: "Nome do perfil" }));
    expect(screen.queryByRole("textbox")).toBeNull();
    expect(onRename).toHaveBeenCalledTimes(2);
  });

  it("confirms before deleting and never deletes the last profile", () => {
    const { props, view } = setup();
    fireEvent.keyDown(card("Anti-ociosidade"), { key: "Delete" });
    expect(screen.getByText(/Excluir este perfil/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(props.onDelete).not.toHaveBeenCalled();
    openMenu("Anti-ociosidade");
    fireEvent.click(screen.getByRole("menuitem", { name: /Excluir/ }));
    fireEvent.click(screen.getByRole("button", { name: "Excluir" }));
    expect(props.onDelete).toHaveBeenCalledWith("c");

    view.unmount();
    setup({ profiles: profiles().slice(0, 1) });
    openMenu("Coleta rápida");
    expect(screen.getByRole("menuitem", { name: /Excluir/ })).toBeDisabled();
    fireEvent.keyDown(card("Coleta rápida"), { key: "Delete" });
    expect(screen.queryByText(/Excluir este perfil/)).toBeNull();
  });

  it("duplicates with Ctrl D and ignores other keys", () => {
    const { props } = setup();
    fireEvent.keyDown(card("Coleta rápida"), { key: "d", ctrlKey: true });
    expect(props.onDuplicate).toHaveBeenCalledWith("a");
    fireEvent.keyDown(card("Coleta rápida"), { key: "x" });
    expect(props.onActivate).not.toHaveBeenCalled();
  });

  it("shows shortcut recording and conflicts inline", () => {
    const { props } = setup({
      recordingProfile: "b",
      heldMods: ["ctrl", "shift"],
      conflict: { target: { kind: "profile", id: "c" }, combo: { mods: [], key: "F6" }, withName: "Iniciar / Parar" },
    });
    expect(within(card("Leitura em loop")).getByText("Shift")).toBeInTheDocument();
    fireEvent.click(card("Leitura em loop"));
    expect(props.onActivate).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toHaveTextContent("Já usado por “Iniciar / Parar”.");
    fireEvent.click(screen.getByRole("button", { name: "Substituir" }));
    fireEvent.click(screen.getByRole("button", { name: "Manter o anterior" }));
    expect(props.onReplaceHotkey).toHaveBeenCalledOnce();
    expect(props.onKeepHotkey).toHaveBeenCalledOnce();
  });

  it("shows notices", () => {
    setup({ notice: { tone: "info", content: "Perfil importado." } });
    expect(screen.getByRole("note")).toHaveTextContent("Perfil importado.");
  });

  it("imports a dropped file and shows the drop hint while dragging", () => {
    const { props, view } = setup();
    const root = view.container.firstElementChild as HTMLElement;
    const file = new File(["{}"], "x.chprofile");
    const transfer = { types: ["Files"], files: [file], dropEffect: "" };
    const over = createEvent.dragOver(root);
    Object.defineProperty(over, "dataTransfer", { value: transfer });
    fireEvent(root, over);
    expect(screen.getByText("Solte o arquivo .chprofile para importar")).toBeInTheDocument();
    const leave = createEvent.dragLeave(root);
    Object.defineProperty(leave, "relatedTarget", { value: null });
    fireEvent(root, leave);
    expect(screen.queryByText("Solte o arquivo .chprofile para importar")).toBeNull();
    const drop = createEvent.drop(root);
    Object.defineProperty(drop, "dataTransfer", { value: transfer });
    fireEvent(root, drop);
    expect(props.onImportFile).toHaveBeenCalledWith(file);
    const text = createEvent.dragOver(root);
    Object.defineProperty(text, "dataTransfer", { value: { types: ["text/plain"], files: [] } });
    fireEvent(root, text);
    expect(screen.queryByText("Solte o arquivo .chprofile para importar")).toBeNull();
  });

  it("shows the empty state of Tela 05.B", () => {
    const { props } = setup({ profiles: [], activeId: null });
    expect(screen.getByText("00 perfis")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Importar" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Criar perfil" }));
    fireEvent.click(screen.getByRole("button", { name: "Importar .chprofile" }));
    expect(props.onCreate).toHaveBeenCalledOnce();
    expect(props.onImport).toHaveBeenCalledOnce();
    expect(screen.getByText("0 perfis · %APPDATA%\\Chenoli\\CH Auto Clicker")).toBeInTheDocument();
  });
});

describe("Header profile dropdown", () => {
  it("switches profiles and opens the library", () => {
    const onSelectProfile = vi.fn();
    const onOpenProfiles = vi.fn();
    render(
      <Header
        profileName="Coleta rápida"
        profiles={profiles()}
        activeProfileId="a"
        onSelectProfile={onSelectProfile}
        onOpenProfiles={onOpenProfiles}
      />,
    );
    const trigger = screen.getByRole("button", { name: /Coleta rápida/ });
    fireEvent.click(trigger);
    expect(screen.getByRole("menuitemradio", { name: "Coleta rápida" })).toHaveAttribute("aria-checked", "true");
    fireEvent.click(screen.getByRole("menuitemradio", { name: "Coleta rápida" }));
    expect(onSelectProfile).not.toHaveBeenCalled();
    fireEvent.click(trigger);
    fireEvent.click(screen.getByRole("menuitemradio", { name: "Leitura em loop" }));
    expect(onSelectProfile).toHaveBeenCalledWith("b");
    fireEvent.click(trigger);
    fireEvent.click(screen.getByRole("menuitem", { name: "Gerenciar perfis…" }));
    expect(onOpenProfiles).toHaveBeenCalledOnce();
    fireEvent.click(trigger);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
    fireEvent.click(trigger);
    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole("menu")).toBeNull();
  });
});

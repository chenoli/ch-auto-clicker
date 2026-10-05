// Kept free of JSX so Playwright can import it. `reference` is the prototype anchor used for parity.
export interface ScreenMeta {
  id: string;
  title: string;
  width: number;
  height: number;
  reference?: string;
}

export const SCREEN_META: ScreenMeta[] = [
  { id: "main-idle", title: "01.A — Janela principal · Click", width: 420, height: 900, reference: "janela" },
  { id: "main-states", title: "10 — Estados do botão principal", width: 1808, height: 900, reference: "estados" },
  { id: "variation", title: "02.B — Variação de intervalo", width: 560, height: 640, reference: "variacao" },
  { id: "tokens", title: "11 — Tokens", width: 880, height: 620 },
];

export const PROTOTYPE_ANCHORS = [
  "janela",
  "pontos",
  "variacao",
  "overlay",
  "hud",
  "perfis",
  "atalhos",
  "temas",
  "config",
  "bandeja",
  "estados",
  "tokens",
] as const;

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
  { id: "points", title: "02.A — Editor de pontos", width: 420, height: 640, reference: "pontos" },
  { id: "variation", title: "02.B — Variação de intervalo", width: 560, height: 640, reference: "variacao" },
  { id: "overlay", title: "03.B — Captura de ponto (50%)", width: 960, height: 540, reference: "overlay" },
  { id: "hotkeys", title: "06 — Atalhos", width: 420, height: 640, reference: "atalhos" },
  { id: "hotkeys-states", title: "06 — Atalhos · gravando e conflito", width: 420, height: 640 },
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

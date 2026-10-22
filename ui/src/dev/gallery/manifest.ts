export interface ScreenMeta {
  id: string;
  title: string;
  width: number;
  height: number;
  reference?: string;
}

export const SCREEN_META: ScreenMeta[] = [
  { id: "main-idle", title: "01.A — Janela principal · Click", width: 420, height: 900, reference: "janela" },
  { id: "main-wiggle", title: "01.B — Janela principal · Wiggle", width: 880, height: 900 },
  { id: "main-scan", title: "01.C — Janela principal · Scan", width: 880, height: 900 },
  { id: "main-states", title: "10 — Estados do botão principal", width: 1808, height: 900, reference: "estados" },
  { id: "points", title: "02.A — Editor de pontos", width: 420, height: 640, reference: "pontos" },
  { id: "variation", title: "02.B — Variação de intervalo", width: 560, height: 640, reference: "variacao" },
  { id: "area-overlay", title: "03.A — Seleção de área (50%)", width: 960, height: 540 },
  { id: "overlay", title: "03.B — Captura de ponto (50%)", width: 960, height: 540, reference: "overlay" },
  { id: "profiles", title: "05.A — Perfis", width: 420, height: 640, reference: "perfis" },
  { id: "profiles-empty", title: "05.B — Perfis · estado vazio", width: 420, height: 640 },
  { id: "hotkeys", title: "06 — Atalhos", width: 420, height: 640, reference: "atalhos" },
  { id: "hotkeys-states", title: "06 — Atalhos · gravando e conflito", width: 420, height: 640 },
  { id: "settings", title: "08 — Configurações", width: 420, height: 640, reference: "config" },
  { id: "hud", title: "04 — Mini HUD", width: 1120, height: 230, reference: "hud" },
  { id: "themes", title: "07 — Tema", width: 420, height: 640, reference: "temas" },
  { id: "tray-menu", title: "09.A — Menu da bandeja", width: 760, height: 480 },
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

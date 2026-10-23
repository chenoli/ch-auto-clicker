export interface ScreenMeta {
  id: string;
  title: string;
  width: number;
  height: number;
  reference?: string;
}

export const SCREEN_META: ScreenMeta[] = [
  { id: "main-idle", title: "01.A — Main window · Click", width: 420, height: 900, reference: "janela" },
  { id: "main-wiggle", title: "01.B — Main window · Wiggle", width: 880, height: 900 },
  { id: "main-scan", title: "01.C — Main window · Scan", width: 880, height: 900 },
  { id: "main-states", title: "10 — Main button states", width: 1808, height: 900, reference: "estados" },
  { id: "points", title: "02.A — Points editor", width: 420, height: 640, reference: "pontos" },
  { id: "variation", title: "02.B — Interval variation", width: 560, height: 640, reference: "variacao" },
  { id: "area-overlay", title: "03.A — Area selection (50%)", width: 960, height: 540 },
  { id: "overlay", title: "03.B — Point capture (50%)", width: 960, height: 540, reference: "overlay" },
  { id: "profiles", title: "05.A — Profiles", width: 420, height: 640, reference: "perfis" },
  { id: "profiles-empty", title: "05.B — Profiles · empty state", width: 420, height: 640 },
  { id: "hotkeys", title: "06 — Hotkeys", width: 420, height: 640, reference: "atalhos" },
  { id: "hotkeys-states", title: "06 — Hotkeys · recording and conflict", width: 420, height: 640 },
  { id: "settings", title: "08 — Settings", width: 420, height: 640, reference: "config" },
  { id: "hud", title: "04 — Mini HUD", width: 1120, height: 350, reference: "hud" },
  { id: "themes", title: "07 — Theme", width: 420, height: 640, reference: "temas" },
  { id: "tray-menu", title: "09.A — Tray menu", width: 760, height: 480 },
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

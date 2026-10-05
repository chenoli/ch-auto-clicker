// Kept free of JSX so Playwright can import it. `reference` is the prototype anchor used for parity.
export interface ScreenMeta {
  id: string;
  title: string;
  width: number;
  height: number;
  reference?: string;
}

export const SCREEN_META: ScreenMeta[] = [{ id: "tokens", title: "11 — Tokens", width: 880, height: 620 }];

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

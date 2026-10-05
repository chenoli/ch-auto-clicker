export const SLIDER_MAX = 2;
export const AMPLITUDE_MAX = 100;

const PREVIEW_WIDTH = 388;
const PREVIEW_SCALE = 40;
const PREVIEW_CAP = 170;

export function parseAmplitude(raw: string): number | null {
  const text = raw.trim().replace("%", "").replace(",", ".").trim();
  if (text === "") return null;
  const n = Number(text);
  if (!Number.isFinite(n)) return null;
  return Math.min(AMPLITUDE_MAX, Math.max(0, Math.round(n * 1000) / 1000));
}

export function formatAmplitude(n: number, locale: string): string {
  return n.toLocaleString(locale, { maximumFractionDigits: n < 10 ? 3 : 2 });
}

export function effectiveAmplitude(pct: number, monitorWidth: number): number {
  return Math.max(1, Math.round((pct / 100) * monitorWidth));
}

export function previewHalfWidth(pct: number): number {
  return Math.min(PREVIEW_CAP, (pct / 100) * PREVIEW_WIDTH * PREVIEW_SCALE);
}

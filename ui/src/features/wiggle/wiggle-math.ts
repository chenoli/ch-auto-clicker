/** The slider covers 0–2 %; the text field accepts up to 100 % (handoff 01.B). */
export const SLIDER_MAX = 2;
export const AMPLITUDE_MAX = 100;

/** Minimap-like preview: 388 px stand for the screen width, scaled ×40, capped at 170 px each side. */
const PREVIEW_WIDTH = 388;
const PREVIEW_SCALE = 40;
const PREVIEW_CAP = 170;

/** "0,05" or "0.05" → 0.05, clamped to 0–100; `null` when it is not a number. */
export function parseAmplitude(raw: string): number | null {
  const text = raw.trim().replace("%", "").replace(",", ".").trim();
  if (text === "") return null;
  const n = Number(text);
  if (!Number.isFinite(n)) return null;
  return Math.min(AMPLITUDE_MAX, Math.max(0, Math.round(n * 1000) / 1000));
}

/** Up to three decimals in the locale ("0,05"; "≈ 0,96 px"). */
export function formatAmplitude(n: number, locale: string): string {
  return n.toLocaleString(locale, { maximumFractionDigits: n < 10 ? 3 : 2 });
}

/** Pixels the cursor really moves: `max(1, round(pct/100 × width))`, same rule as Rust. */
export function effectiveAmplitude(pct: number, monitorWidth: number): number {
  return Math.max(1, Math.round((pct / 100) * monitorWidth));
}

/** Half the width of the animated band in the preview. */
export function previewHalfWidth(pct: number): number {
  return Math.min(PREVIEW_CAP, (pct / 100) * PREVIEW_WIDTH * PREVIEW_SCALE);
}

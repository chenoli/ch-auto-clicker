const formats = new Map<string, Intl.NumberFormat>();

function numberFormat(locale: string, options: Intl.NumberFormatOptions): Intl.NumberFormat {
  const key = `${locale}|${JSON.stringify(options)}`;
  let fmt = formats.get(key);
  if (!fmt) {
    fmt = new Intl.NumberFormat(locale, options);
    formats.set(key, fmt);
  }
  return fmt;
}

export function formatCps(cps: number, locale: string): string {
  return numberFormat(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(cps);
}

export function cpsFromInterval(intervalMs: number, double: boolean): number {
  return (1000 / intervalMs) * (double ? 2 : 1);
}

export function formatCount(n: number, locale: string): string {
  return numberFormat(locale, { maximumFractionDigits: 0 }).format(n);
}

export function formatOneDecimal(n: number, locale: string): string {
  return numberFormat(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(n);
}

export function formatElapsed(ms: number): string {
  const tenths = Math.floor(Math.max(0, ms) / 100);
  const minutes = Math.floor(tenths / 600);
  const seconds = Math.floor(tenths / 10) % 60;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}.${tenths % 10}`;
}

export function formatDuration(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const hh = Math.floor(s / 3600);
  const mm = Math.floor(s / 60) % 60;
  const ss = s % 60;
  return [hh, mm, ss].map((v) => String(v).padStart(2, "0")).join(":");
}

export function countdownSeconds(remainingMs: number): number {
  return Math.max(1, Math.ceil(remainingMs / 1000));
}

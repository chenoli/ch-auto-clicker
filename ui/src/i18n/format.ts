const oneDecimal = new Map<string, Intl.NumberFormat>();

function decimalFormat(locale: string): Intl.NumberFormat {
  let fmt = oneDecimal.get(locale);
  if (!fmt) {
    fmt = new Intl.NumberFormat(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 });
    oneDecimal.set(locale, fmt);
  }
  return fmt;
}

export function formatCps(cps: number, locale: string): string {
  return decimalFormat(locale).format(cps);
}

export function cpsFromInterval(intervalMs: number, double: boolean): number {
  return (1000 / intervalMs) * (double ? 2 : 1);
}

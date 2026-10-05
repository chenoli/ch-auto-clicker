import type { Profile } from "../../ipc";
import type { TFunction } from "i18next";

/** One line that says what a profile does, as on the Tela 05 cards. */
export function profileSummary(profile: Profile, locale: string, t: TFunction): string {
  const { click, modes } = profile;
  if (profile.mode === "wiggle") {
    const pct = (modes.wiggle.amplitudePct ?? 0).toLocaleString(locale, { maximumFractionDigits: 3 });
    return [
      `${pct}%`,
      `${modes.wiggle.speedHz} Hz`,
      ...(modes.wiggle.clickWhileWiggling ? [t("profiles.summary.clicks")] : []),
    ].join(" · ");
  }
  if (profile.mode === "scan") {
    const area = modes.scan.area;
    return [
      area ? `${area.w}×${area.h}` : t("profiles.summary.noArea"),
      `${modes.scan.lineSpacingPx} px`,
      modes.scan.loop ? t("profiles.summary.loop") : t("profiles.summary.once"),
    ].join(" · ");
  }
  const interval = click.variation.enabled
    ? t("profiles.summary.variation", { blocks: click.variation.blocksMs.join("·") })
    : `${click.intervalMs} ms`;
  return [
    interval,
    t(`wiggle.button.${click.button}`),
    ...(click.kind === "double" ? [t("wiggle.double")] : []),
    ...(modes.click.position === "points" ? [t("profiles.summary.points", { count: modes.click.points.length })] : []),
  ].join(" · ");
}

/** Case- and accent-insensitive name search. */
export function matchesSearch(name: string, query: string): boolean {
  const fold = (s: string) =>
    s
      .normalize("NFD")
      .replace(/\p{Diacritic}/gu, "")
      .toLowerCase();
  return fold(name).includes(fold(query.trim()));
}

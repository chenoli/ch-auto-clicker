import { useTranslation } from "react-i18next";

import { cpsFromInterval, formatCps } from "../../i18n/format";

import type { Profile } from "../../ipc";
import type { TFunction } from "i18next";
import type { ReactElement } from "react";

/** "100 ms · 10,0 CPS · esquerdo": the Click tab settings that Wiggle and Scan click with. */
export function rateSummary(profile: Profile, locale: string, t: TFunction): string {
  const click = profile.click;
  const double = click.kind === "double";
  return [
    `${click.intervalMs} ms`,
    `${formatCps(cpsFromInterval(click.intervalMs, double), locale)} ${t("click.cps")}`,
    t(`wiggle.button.${click.button}`),
    ...(double ? [t("wiggle.double")] : []),
  ].join(" · ");
}

/** Card with the inherited click rate and "Ajustar ›", which switches to the Click tab (D1). */
export function InheritedRate({
  profile,
  locale,
  onAdjust,
}: {
  profile: Profile;
  locale: string;
  onAdjust: () => void;
}): ReactElement {
  const { t } = useTranslation();
  return (
    <div className="inherited-rate">
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="text-12 font-semibold text-fg tabular-nums">{rateSummary(profile, locale, t)}</span>
        <span className="text-11 leading-snug text-fg-3">{t("wiggle.rateFromClick")}</span>
      </div>
      <button type="button" className="link flex-none" onClick={onAdjust}>
        {t("wiggle.adjust")}
      </button>
    </div>
  );
}

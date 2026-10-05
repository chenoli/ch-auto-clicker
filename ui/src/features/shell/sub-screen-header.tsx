import { useTranslation } from "react-i18next";

import type { ReactElement, ReactNode } from "react";

interface SubScreenHeaderProps {
  kicker: string;
  title: string;
  onBack: () => void;
  /** Text action on the right ("Restaurar padrão"). */
  action?: ReactNode;
}

/** Header of a sub-screen: "‹" back button, kicker and Syne title (handoff 02.A, 05, 06, 08). */
export function SubScreenHeader({ kicker, title, onBack, action }: SubScreenHeaderProps): ReactElement {
  const { t } = useTranslation();
  return (
    <div className="flex flex-none items-center gap-3 px-4 pt-3.5 pb-3">
      <button type="button" className="back-button" aria-label={t("common.back")} onClick={onBack}>
        ‹
      </button>
      <div className="flex flex-1 flex-col gap-0.75">
        <span className="micro-label">{kicker}</span>
        <h1 className="m-0 font-display text-20 leading-none font-extrabold tracking-logo text-fg">{title}</h1>
      </div>
      {action}
    </div>
  );
}

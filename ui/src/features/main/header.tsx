import { useTranslation } from "react-i18next";

import type { ReactElement } from "react";

interface HeaderProps {
  profileName: string;
  onOpenProfiles?: () => void;
  onOpenSettings?: () => void;
}

export function Header({ profileName, onOpenProfiles, onOpenSettings }: HeaderProps): ReactElement {
  const { t } = useTranslation();
  return (
    <div className="flex flex-none items-center justify-between px-4 pt-3 pb-2.5">
      <button
        type="button"
        className="flex h-9 items-center gap-2.5 rounded-control border border-border bg-bg-2 px-3 text-left"
      >
        <span className="size-1.5 rounded-full bg-accent" />
        <span className="flex flex-col leading-label">
          <span className="micro-label">{t("header.profile")}</span>
          <span className="text-13 font-semibold text-fg">{profileName}</span>
        </span>
        <span className="pl-2 text-9 text-fg-3">▼</span>
      </button>
      <div className="flex gap-1.5">
        <button type="button" className="icon-button" aria-label={t("header.profiles")} onClick={onOpenProfiles}>
          <span className="icon-bars">
            <span />
            <span />
            <span />
          </span>
        </button>
        <button type="button" className="icon-button" aria-label={t("header.settings")} onClick={onOpenSettings}>
          <span className="icon-ring">
            <span />
          </span>
        </button>
      </div>
    </div>
  );
}

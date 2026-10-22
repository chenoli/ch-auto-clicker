import { useEffect, useRef, useState } from "react";

import { useTranslation } from "react-i18next";

import type { ReactElement } from "react";

const GEAR_TEETH = [0, 45, 90, 135, 180, 225, 270, 315].map((deg) => {
  const rad = (deg * Math.PI) / 180;
  const at = (r: number) => ({ x: 8 + r * Math.cos(rad), y: 8 + r * Math.sin(rad) });
  return { deg, from: at(4.4), to: at(6.6) };
});

interface HeaderProps {
  profileName: string;
  profiles?: readonly { id: string; name: string }[];
  activeProfileId?: string | null;
  onSelectProfile?: ((id: string) => void) | undefined;
  onOpenProfiles?: (() => void) | undefined;
  onOpenSettings?: (() => void) | undefined;
}

export function Header({
  profileName,
  profiles = [],
  activeProfileId = null,
  onSelectProfile,
  onOpenProfiles,
  onOpenSettings,
}: HeaderProps): ReactElement {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: PointerEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("pointerdown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointerdown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="flex flex-none items-center justify-between px-4 pt-3 pb-2.5">
      <div ref={rootRef} className="relative">
        <button
          type="button"
          className="flex h-9 items-center gap-2.5 rounded-control border border-border bg-bg-2 px-3 text-left"
          aria-haspopup="menu"
          aria-expanded={open}
          onClick={() => setOpen((o) => !o)}
        >
          <span className="size-1.5 rounded-full bg-accent" />
          <span className="flex flex-col leading-label">
            <span className="micro-label">{t("header.profile")}</span>
            <span className="max-w-50 truncate text-13 font-semibold text-fg">{profileName}</span>
          </span>
          <span className="pl-2 text-9 text-fg-3">▼</span>
        </button>
        {open && (
          <div className="popover absolute top-full left-0 z-20 w-56" role="menu" aria-label={t("header.profiles")}>
            <div className="max-h-60 overflow-auto">
              {profiles.map((p) => (
                <button
                  key={p.id}
                  type="button"
                  role="menuitemradio"
                  aria-checked={p.id === activeProfileId}
                  className="menu-item"
                  onClick={() => {
                    setOpen(false);
                    if (p.id !== activeProfileId) onSelectProfile?.(p.id);
                  }}
                >
                  <span className="truncate">{p.name}</span>
                </button>
              ))}
            </div>
            <div className="menu-divider" />
            <button
              type="button"
              role="menuitem"
              className="menu-item"
              onClick={() => {
                setOpen(false);
                onOpenProfiles?.();
              }}
            >
              {t("header.manageProfiles")}
            </button>
          </div>
        )}
      </div>
      <div className="flex gap-1.5">
        <button type="button" className="icon-button" aria-label={t("header.profiles")} onClick={onOpenProfiles}>
          <span className="icon-bars">
            <span />
            <span />
            <span />
          </span>
        </button>
        <button type="button" className="icon-button" aria-label={t("header.settings")} onClick={onOpenSettings}>
          <svg className="icon-gear" viewBox="0 0 16 16" aria-hidden="true">
            <circle cx="8" cy="8" r="3.9" />
            <circle cx="8" cy="8" r="1.4" />
            {GEAR_TEETH.map(({ deg, from, to }) => (
              <line key={deg} x1={from.x} y1={from.y} x2={to.x} y2={to.y} />
            ))}
          </svg>
        </button>
      </div>
    </div>
  );
}

import { useEffect, useRef, useState } from "react";

import { useTranslation } from "react-i18next";

import type { EngineState } from "../../ipc";
import type { KeyboardEvent as ReactKeyboardEvent, ReactElement } from "react";

export type TrayAction =
  "start" | "pause" | "show" | "hud" | "hud-reset" | "settings" | "support" | "quit" | `profile:${string}`;

export interface TrayMenuViewProps {
  state: EngineState;
  profileName: string;
  profiles: readonly { id: string; name: string }[];
  activeProfileId: string | null;
  showHud: boolean;
  startHint: string;
  pauseHint: string;
  initialSubmenu?: boolean;
  onAction: (action: TrayAction) => void;
  onClose: () => void;
}

type Row =
  | { kind: "item"; action: TrayAction; label: string; hint?: string; disabled?: boolean; tone?: "support" | "muted" }
  | { kind: "profile" }
  | { kind: "check"; action: TrayAction; label: string; checked: boolean }
  | { kind: "divider" };

const ACTIVE: readonly EngineState[] = ["countdown", "running", "paused"];

export function stateKey(state: EngineState): "idle" | "countdown" | "running" | "paused" | "error" {
  return state === "stopping" ? "idle" : state;
}

export function TrayMenuView(props: TrayMenuViewProps): ReactElement {
  const { t } = useTranslation();
  const { state, profiles } = props;
  const [focused, setFocused] = useState(0);
  const [submenu, setSubmenu] = useState(props.initialSubmenu ?? false);
  const [subFocused, setSubFocused] = useState(0);
  const menuRef = useRef<HTMLDivElement>(null);

  const active = ACTIVE.includes(state);
  const pausable = state === "running" || state === "paused";
  const rows: Row[] = [
    { kind: "item", action: "start", label: t(active ? "tray.stop" : "tray.start"), hint: props.startHint },
    {
      kind: "item",
      action: "pause",
      label: t(state === "paused" ? "tray.resume" : "tray.pause"),
      hint: props.pauseHint,
      disabled: !pausable,
    },
    { kind: "profile" },
    { kind: "divider" },
    { kind: "item", action: "show", label: t("tray.showWindow") },
    { kind: "check", action: "hud", label: t("tray.showHud"), checked: props.showHud },
    { kind: "item", action: "hud-reset", label: t("tray.resetHud") },
    { kind: "item", action: "settings", label: t("tray.settings") },
    { kind: "item", action: "support", label: t("tray.support"), hint: "↗", tone: "support" },
    { kind: "divider" },
    { kind: "item", action: "quit", label: t("tray.quit"), tone: "muted" },
  ];
  const focusable = rows
    .map((row, i) => ({ row, i }))
    .filter(({ row }) => row.kind !== "divider" && !(row.kind === "item" && row.disabled))
    .map(({ i }) => i);
  const profileRow = rows.findIndex((r) => r.kind === "profile");
  const activeIndex = Math.max(
    0,
    profiles.findIndex((p) => p.id === props.activeProfileId),
  );

  useEffect(() => {
    menuRef.current?.focus();
  }, []);

  const run = (row: Row) => {
    if (row.kind === "item" || row.kind === "check") props.onAction(row.action);
    else if (row.kind === "profile") {
      setSubmenu(true);
      setSubFocused(activeIndex);
    }
  };
  const step = (delta: number) => {
    const at = focusable.indexOf(focused);
    const next = focusable[(at + delta + focusable.length) % focusable.length];
    if (next !== undefined) setFocused(next);
  };

  const onKeyDown = (e: ReactKeyboardEvent) => {
    const key = e.key;
    if (key === "Escape") {
      if (submenu) setSubmenu(false);
      else props.onClose();
    } else if (submenu) {
      if (key === "ArrowDown") setSubFocused((i) => (i + 1) % Math.max(1, profiles.length));
      else if (key === "ArrowUp") setSubFocused((i) => (i - 1 + profiles.length) % Math.max(1, profiles.length));
      else if (key === "ArrowRight") setSubmenu(false);
      else if (key === "Enter" || key === " ") {
        const p = profiles[subFocused];
        if (p) props.onAction(`profile:${p.id}`);
      } else return;
    } else if (key === "ArrowDown") step(1);
    else if (key === "ArrowUp") step(-1);
    else if (key === "ArrowLeft" && focused === profileRow) run({ kind: "profile" });
    else if (key === "Enter" || key === " ") {
      const row = rows[focused];
      if (row) run(row);
    } else return;
    e.preventDefault();
  };

  return (
    <div
      ref={menuRef}
      className="tray-menu"
      role="menu"
      aria-label={t("tray.menu")}
      tabIndex={-1}
      onKeyDown={onKeyDown}
    >
      <div className="tray-menu__header">
        <span className="font-display text-15 font-extrabold text-fg">
          CH<span className="text-accent">.</span>
        </span>
        <div className="flex min-w-0 flex-1 flex-col gap-px">
          <span className="truncate text-12 font-semibold text-fg">{props.profileName}</span>
          <span className="tray-menu__state">{t(`tray.state.${stateKey(state)}`)}</span>
        </div>
      </div>
      <div className="menu-divider tray-menu__first-divider" />
      {rows.map((row, i) => {
        if (row.kind === "divider") return <div key={`d${i}`} className="menu-divider" />;
        const isFocused = focused === i && !submenu;
        if (row.kind === "profile") {
          return (
            <div key="profile" className="relative">
              <button
                type="button"
                role="menuitem"
                aria-haspopup="menu"
                aria-expanded={submenu}
                className="tray-item"
                data-focused={isFocused || submenu}
                onPointerEnter={() => {
                  setFocused(i);
                  setSubmenu(true);
                  setSubFocused(activeIndex);
                }}
                onClick={() => run(row)}
              >
                {t("tray.profile")}
                <span className="tray-item__hint">›</span>
              </button>
              {submenu && (
                <div className="tray-menu tray-submenu" role="menu" aria-label={t("tray.profile")}>
                  {profiles.map((p, j) => (
                    <button
                      key={p.id}
                      type="button"
                      role="menuitemradio"
                      aria-checked={p.id === props.activeProfileId}
                      className="tray-item tray-item--sub"
                      data-focused={subFocused === j}
                      onPointerEnter={() => setSubFocused(j)}
                      onClick={() => props.onAction(`profile:${p.id}`)}
                    >
                      <span className="tray-item__check">{p.id === props.activeProfileId ? "✓" : ""}</span>
                      <span className="truncate">{p.name}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          );
        }
        return (
          <button
            key={row.action}
            type="button"
            role={row.kind === "check" ? "menuitemcheckbox" : "menuitem"}
            aria-checked={row.kind === "check" ? row.checked : undefined}
            className="tray-item"
            data-tone={row.kind === "item" ? row.tone : undefined}
            data-focused={isFocused}
            disabled={row.kind === "item" && row.disabled}
            onPointerEnter={() => {
              setFocused(i);
              setSubmenu(false);
            }}
            onClick={() => run(row)}
          >
            {row.label}
            {row.kind === "check"
              ? row.checked && <span className="tray-item__check-end">✓</span>
              : row.hint && <span className="tray-item__hint">{row.hint}</span>}
          </button>
        );
      })}
    </div>
  );
}

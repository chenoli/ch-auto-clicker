import { useEffect, useRef, useState } from "react";

import { useTranslation } from "react-i18next";

import { Kbd } from "../../components/kbd";

import { hotkeyLabel } from "../hotkeys/format-hotkey";

import type { Modifier, Profile } from "../../ipc";
import type { ReactElement } from "react";

const MOD_LABELS: Record<Modifier, string> = { ctrl: "Ctrl", alt: "Alt", shift: "Shift", win: "Win" };

export type CardMode = "normal" | "renaming" | "recording" | "confirmDelete";

export interface ProfileCardProps {
  profile: Profile;
  summary: string;
  active: boolean;
  mode: CardMode;
  menuOpen: boolean;
  canDelete: boolean;
  heldMods?: readonly Modifier[];
  renameError?: string | null;
  conflictText?: string | null;
  onActivate: () => void;
  onMenu: (open: boolean) => void;
  onStartRename: () => void;
  onRename: (name: string) => void;
  onCancel: () => void;
  onRecordHotkey: () => void;
  onClearHotkey: () => void;
  onDuplicate: () => void;
  onExport: () => void;
  onAskDelete: () => void;
  onDelete: () => void;
  onReplaceHotkey: () => void;
  onKeepHotkey: () => void;
}

function ModeThumb({ profile }: { profile: Profile }) {
  return (
    <div className="profile-thumb" aria-hidden="true">
      {profile.mode === "click" &&
        [
          [12, 12],
          [40, 18],
          [30, 34],
          [16, 30],
        ].map(([x, y]) => <span key={`${x}-${y}`} className="profile-thumb__dot" style={{ left: x, top: y }} />)}
      {profile.mode === "wiggle" && (
        <>
          <span className="profile-thumb__track" />
          <span className="profile-thumb__band" />
        </>
      )}
      {profile.mode === "scan" && (
        <svg width="64" height="48" className="absolute inset-0">
          <polyline
            points="8,8 56,8 56,16 8,16 8,24 56,24 56,32 8,32 8,40 56,40"
            fill="none"
            stroke="var(--accent)"
            strokeWidth="1.2"
          />
        </svg>
      )}
    </div>
  );
}

function RenameField({
  name,
  onRename,
  onCancel,
}: {
  name: string;
  onRename: (n: string) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation();
  const ref = useRef<HTMLInputElement>(null);
  const [draft, setDraft] = useState(name);
  useEffect(() => {
    ref.current?.select();
  }, []);
  return (
    <input
      ref={ref}
      className="profile-rename"
      aria-label={t("profiles.renameLabel")}
      value={draft}
      maxLength={48}
      onChange={(e) => setDraft(e.target.value)}
      onClick={(e) => e.stopPropagation()}
      onKeyDown={(e) => {
        e.stopPropagation();
        if (e.key === "Enter") onRename(draft);
        if (e.key === "Escape") onCancel();
      }}
      onBlur={() => (draft.trim() && draft !== name ? onRename(draft) : onCancel())}
    />
  );
}

export function ProfileCard(props: ProfileCardProps): ReactElement {
  const { t } = useTranslation();
  const { profile, mode, menuOpen } = props;
  const menu = (
    label: string,
    hint: string,
    action: () => void,
    extra: { danger?: boolean; disabled?: boolean } = {},
  ) => (
    <button
      type="button"
      role="menuitem"
      className={`menu-item justify-between ${extra.danger ? "menu-item--danger" : ""}`}
      disabled={extra.disabled}
      onClick={(e) => {
        e.stopPropagation();
        props.onMenu(false);
        action();
      }}
    >
      {label}
      <span className="text-11 text-fg-3">{hint}</span>
    </button>
  );

  return (
    <div className="relative">
      <div
        role="button"
        tabIndex={0}
        aria-current={props.active ? "true" : undefined}
        aria-label={profile.name}
        className="profile-card"
        data-active={props.active}
        onClick={() => mode === "normal" && props.onActivate()}
        onKeyDown={(e) => {
          if (mode !== "normal" || e.target !== e.currentTarget) return;
          if (e.key === "Enter" || e.key === " ") props.onActivate();
          else if (e.key === "F2") props.onStartRename();
          else if (e.key === "Delete") props.onAskDelete();
          else if (e.ctrlKey && e.key.toLowerCase() === "d") props.onDuplicate();
          else return;
          e.preventDefault();
        }}
      >
        <ModeThumb profile={profile} />
        <div className="flex min-w-0 flex-col gap-1">
          <div className="flex min-w-0 items-center gap-2">
            {mode === "renaming" ? (
              <RenameField name={profile.name} onRename={props.onRename} onCancel={props.onCancel} />
            ) : (
              <span className="truncate text-14 font-semibold text-fg">{profile.name}</span>
            )}
            <span className="mode-badge">{t(`modes.${profile.mode}`)}</span>
          </div>
          {mode === "confirmDelete" ? (
            <span className="text-11 text-danger">{t("profiles.confirmDelete")}</span>
          ) : props.renameError ? (
            <span className="text-11 text-danger">{props.renameError}</span>
          ) : (
            <span className="truncate text-11 text-fg-3 tabular-nums">{props.summary}</span>
          )}
        </div>
        <div className="flex items-center gap-1.5">
          {mode === "confirmDelete" ? (
            <>
              <button
                type="button"
                className="profile-action profile-action--danger"
                onClick={(e) => {
                  e.stopPropagation();
                  props.onDelete();
                }}
              >
                {t("profiles.delete")}
              </button>
              <button
                type="button"
                className="profile-action"
                onClick={(e) => {
                  e.stopPropagation();
                  props.onCancel();
                }}
              >
                {t("common.cancel")}
              </button>
            </>
          ) : (
            <>
              {mode === "recording" ? (
                <span className="profile-key profile-key--recording">
                  {props.heldMods?.length
                    ? props.heldMods.map((m) => <Kbd key={m}>{MOD_LABELS[m]}</Kbd>)
                    : t("hotkeys.recording")}
                </span>
              ) : (
                <span className="profile-key" data-empty={!profile.hotkey}>
                  {hotkeyLabel(profile.hotkey)}
                </span>
              )}
              <button
                type="button"
                className="profile-more"
                aria-label={t("profiles.actions", { name: profile.name })}
                aria-haspopup="menu"
                aria-expanded={menuOpen}
                data-open={menuOpen}
                onClick={(e) => {
                  e.stopPropagation();
                  props.onMenu(!menuOpen);
                }}
              >
                ⋯
              </button>
            </>
          )}
        </div>
      </div>
      {props.conflictText && (
        <div className="profile-conflict" role="alert">
          <span>{props.conflictText}</span>
          <button type="button" className="link" onClick={props.onReplaceHotkey}>
            {t("hotkeys.replace")}
          </button>
          <button type="button" className="link link--muted" onClick={props.onKeepHotkey}>
            {t("hotkeys.keep")}
          </button>
        </div>
      )}
      {menuOpen && (
        <div className="popover profile-menu" role="menu" aria-label={t("profiles.actions", { name: profile.name })}>
          {menu(t("profiles.rename"), "F2", props.onStartRename)}
          {menu(t("profiles.setHotkey"), "", props.onRecordHotkey)}
          {profile.hotkey && menu(t("profiles.clearHotkey"), hotkeyLabel(profile.hotkey), props.onClearHotkey)}
          {menu(t("profiles.duplicate"), "Ctrl D", props.onDuplicate)}
          {menu(t("profiles.export"), ".chprofile", props.onExport)}
          <div className="menu-divider" />
          {menu(t("profiles.delete"), "Del", props.onAskDelete, { danger: true, disabled: !props.canDelete })}
        </div>
      )}
    </div>
  );
}

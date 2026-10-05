import { useEffect, useState } from "react";

import { useTranslation } from "react-i18next";

import { Button } from "../../components/button";
import { Callout } from "../../components/callout";

import { TitleBar } from "../main/title-bar";
import { SubScreenHeader } from "../shell/sub-screen-header";
import { ProfileCard } from "./profile-card";
import { matchesSearch, profileSummary } from "./profile-summary";

import type { Modifier, Profile } from "../../ipc";
import type { PendingConflict } from "../hotkeys/hotkeys-screen";
import type { CardMode } from "./profile-card";
import type { DragEvent, ReactElement, ReactNode } from "react";

export interface ProfilesScreenProps {
  profiles: readonly Profile[];
  activeId: string | null;
  locale: string;
  /** Where the profiles live, for the footer. */
  dataPath: string;
  recordingProfile: string | null;
  heldMods: readonly Modifier[];
  conflict: PendingConflict | null;
  /** Result of the last import or a failed command. */
  notice?: { tone: "info" | "warn" | "danger"; content: ReactNode } | null;
  onBack: () => void;
  onMinimize: () => void;
  onClose: () => void;
  onActivate: (id: string) => void;
  onCreate: () => void;
  /** Resolves with an error code when the name is refused. */
  onRename: (id: string, name: string) => Promise<string | null>;
  onDuplicate: (id: string) => void;
  onExport: (id: string) => void;
  onDelete: (id: string) => void;
  onImport: () => void;
  onImportFile: (file: File) => void;
  onRecordHotkey: (id: string) => void;
  onClearHotkey: (id: string) => void;
  onCancelRecord: () => void;
  onReplaceHotkey: () => void;
  onKeepHotkey: () => void;
  /** Gallery: open this card's menu on first render. */
  initialMenuFor?: string | null;
}

const hasFiles = (e: DragEvent) => Array.from(e.dataTransfer.types).includes("Files");

/** Tela 05.B: no profiles yet; also the drop target for `.chprofile` files. */
export function EmptyProfiles({ onCreate, onImport }: { onCreate: () => void; onImport: () => void }): ReactElement {
  const { t } = useTranslation();
  return (
    <div className="profiles-empty">
      <span className="micro-label">{t("profiles.emptyCount")}</span>
      <span className="font-display text-28 leading-display font-extrabold tracking-display text-fg">
        {t("profiles.emptyTitle1")}
        <br />
        {t("profiles.emptyTitle2")}
      </span>
      <span className="max-w-70 text-13 leading-relaxed text-fg-2">{t("profiles.emptyText")}</span>
      <div className="flex w-60 flex-col gap-2 pt-1.5">
        <Button variant="primary" className="h-10.5" onClick={onCreate}>
          {t("profiles.create")}
        </Button>
        <Button className="h-10.5" onClick={onImport}>
          {t("profiles.importFile")}
        </Button>
      </div>
      <span className="text-11 text-fg-3">{t("profiles.dropHint")}</span>
    </div>
  );
}

/** Tela 05.A: the profile library. */
export function ProfilesScreen(props: ProfilesScreenProps): ReactElement {
  const { t } = useTranslation();
  const { profiles, activeId, conflict } = props;
  const [query, setQuery] = useState("");
  const [menuFor, setMenuFor] = useState<string | null>(props.initialMenuFor ?? null);
  const [cardMode, setCardMode] = useState<{ id: string; mode: CardMode } | null>(null);
  const [renameError, setRenameError] = useState<{ id: string; code: string } | null>(null);
  const [dropping, setDropping] = useState(false);

  // Close the "⋯" menu on any outside click or Esc.
  useEffect(() => {
    if (!menuFor) return;
    const close = () => setMenuFor(null);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();
    const timer = window.setTimeout(() => window.addEventListener("click", close), 0);
    window.addEventListener("keydown", onKey);
    return () => {
      window.clearTimeout(timer);
      window.removeEventListener("click", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [menuFor]);

  const shown = profiles.filter((p) => matchesSearch(p.name, query));
  const modeOf = (id: string): CardMode =>
    props.recordingProfile === id ? "recording" : cardMode?.id === id ? cardMode.mode : "normal";
  const end = () => {
    setCardMode(null);
    setRenameError(null);
  };
  const rename = async (id: string, name: string) => {
    const code = await props.onRename(id, name);
    if (code) setRenameError({ id, code });
    else end();
  };
  const active = profiles.find((p) => p.id === activeId);

  return (
    <div
      className="relative flex h-full flex-col bg-bg"
      onDragOver={(e) => {
        if (!hasFiles(e)) return;
        e.preventDefault();
        e.dataTransfer.dropEffect = "copy";
        if (!dropping) setDropping(true);
      }}
      onDragLeave={(e) => {
        if (e.currentTarget === e.target || !e.currentTarget.contains(e.relatedTarget as Node)) setDropping(false);
      }}
      onDrop={(e) => {
        if (!hasFiles(e)) return;
        e.preventDefault();
        setDropping(false);
        const file = e.dataTransfer.files[0];
        if (file) props.onImportFile(file);
      }}
    >
      <TitleBar onMinimize={props.onMinimize} onClose={props.onClose} />
      <SubScreenHeader
        title={t("profiles.title")}
        onBack={props.onBack}
        action={
          profiles.length > 0 && (
            <div className="flex">
              <button type="button" className="text-button" onClick={props.onImport}>
                {t("profiles.import")}
              </button>
              <button
                type="button"
                className="text-button"
                disabled={!active}
                onClick={() => active && props.onExport(active.id)}
              >
                {t("profiles.exportActive")}
              </button>
            </div>
          )
        }
      />
      {profiles.length === 0 ? (
        <EmptyProfiles onCreate={props.onCreate} onImport={props.onImport} />
      ) : (
        <>
          <div className="flex flex-none gap-2 px-4 pb-3.5">
            <label className="search-field">
              <span className="search-field__icon" aria-hidden="true" />
              <input
                type="search"
                placeholder={t("profiles.search")}
                aria-label={t("profiles.search")}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
            </label>
            <Button variant="primary" className="h-9 px-3.5" onClick={props.onCreate}>
              {t("profiles.new")}
            </Button>
          </div>
          {props.notice && (
            <div className="flex-none px-4 pb-3">
              <Callout tone={props.notice.tone}>{props.notice.content}</Callout>
            </div>
          )}
          <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-auto px-4 pb-3">
            {shown.length === 0 && <p className="m-0 py-6 text-center text-12 text-fg-3">{t("profiles.noMatch")}</p>}
            {shown.map((p) => (
              <ProfileCard
                key={p.id}
                profile={p}
                summary={profileSummary(p, props.locale, t)}
                active={p.id === activeId}
                mode={modeOf(p.id)}
                menuOpen={menuFor === p.id}
                canDelete={profiles.length > 1}
                heldMods={props.heldMods}
                renameError={
                  renameError?.id === p.id
                    ? t(`errors.${renameError.code}`, { defaultValue: t("errors.generic", { code: renameError.code }) })
                    : null
                }
                conflictText={
                  conflict?.target.kind === "profile" && conflict.target.id === p.id
                    ? t("hotkeys.conflictShort", { name: conflict.withName })
                    : null
                }
                onActivate={() => props.onActivate(p.id)}
                onMenu={(open) => setMenuFor(open ? p.id : null)}
                onStartRename={() => setCardMode({ id: p.id, mode: "renaming" })}
                onRename={(name) => void rename(p.id, name)}
                onCancel={() => {
                  if (props.recordingProfile === p.id) props.onCancelRecord();
                  end();
                }}
                onRecordHotkey={() => props.onRecordHotkey(p.id)}
                onClearHotkey={() => props.onClearHotkey(p.id)}
                onDuplicate={() => props.onDuplicate(p.id)}
                onExport={() => props.onExport(p.id)}
                onAskDelete={() => profiles.length > 1 && setCardMode({ id: p.id, mode: "confirmDelete" })}
                onDelete={() => {
                  end();
                  props.onDelete(p.id);
                }}
                onReplaceHotkey={props.onReplaceHotkey}
                onKeepHotkey={props.onKeepHotkey}
              />
            ))}
          </div>
        </>
      )}
      <div className="flex h-9 flex-none items-center border-t border-divider px-4 text-11 text-fg-3">
        <span className="truncate">{t("profiles.footer", { count: profiles.length, path: props.dataPath })}</span>
      </div>
      {dropping && (
        <div className="profiles-drop" aria-hidden="true">
          {t("profiles.dropNow")}
        </div>
      )}
    </div>
  );
}

import { useState } from "react";

import { useTranslation } from "react-i18next";

import { commands } from "../../ipc";

import { useAppStore } from "../../store/app-store";

import { hotkeyLabel } from "../hotkeys/format-hotkey";
import { ProfilesScreen } from "./profiles-screen";

import type { AppError, ImportResult } from "../../ipc";
import type { HotkeyRecordingApi } from "../hotkeys/use-hotkey-recording";
import type { ReactNode } from "react";

/** Same limit as Rust (`chprofile::MAX_BYTES`): checked before reading a dropped file. */
const MAX_IMPORT_BYTES = 1024 * 1024;
export const DATA_PATH = "%APPDATA%\\Chenoli\\CH Clicker";

type Notice = { tone: "info" | "warn" | "danger"; content: ReactNode } | null;

interface ProfilesRouteProps {
  recording: HotkeyRecordingApi;
  onBack: () => void;
  onMinimize: () => void;
  onClose: () => void;
}

/** Tela 05 connected to Rust: every action is a command; the list comes back as `profiles-changed`. */
export function ProfilesRoute({ recording, onBack, onMinimize, onClose }: ProfilesRouteProps) {
  const { t } = useTranslation();
  const profiles = useAppStore((s) => s.profiles);
  const settings = useAppStore((s) => s.settings);
  const [notice, setNotice] = useState<Notice>(null);

  const fail = (error: AppError) =>
    setNotice({
      tone: "danger",
      content: t(`errors.${error.code}`, { defaultValue: t("errors.generic", { code: error.code }) }),
    });
  const run = async <T,>(call: Promise<{ status: "ok"; data: T } | { status: "error"; error: AppError }>) => {
    setNotice(null);
    const result = await call;
    if (result.status === "error") {
      fail(result.error);
      return null;
    }
    return result;
  };

  const imported = (result: ImportResult) => {
    const notes = result.notes.map((n) =>
      n.kind === "renamed"
        ? t("profiles.notes.renamed", { to: n.to })
        : n.kind === "hotkeyDropped"
          ? t("profiles.notes.hotkeyDropped", { combo: hotkeyLabel(n.combo) })
          : t("profiles.notes.migrated"),
    );
    setNotice({
      tone: notes.length ? "warn" : "info",
      content: (
        <>
          {t("profiles.imported", { name: result.profile.name })}
          {notes.map((n) => (
            <span key={n} className="block">
              {n}
            </span>
          ))}
        </>
      ),
    });
  };

  return (
    <ProfilesScreen
      profiles={profiles}
      activeId={settings?.activeProfileId ?? null}
      locale={settings?.locale ?? "pt-BR"}
      dataPath={DATA_PATH}
      recordingProfile={recording.recordingProfile}
      heldMods={recording.heldMods}
      conflict={recording.conflict}
      notice={notice}
      onBack={onBack}
      onMinimize={onMinimize}
      onClose={onClose}
      onActivate={(id) => void run(commands.profileActivate(id))}
      onCreate={() => void run(commands.profileCreate(t("profiles.defaultName")))}
      onRename={async (id, name) => {
        const result = await commands.profileRename(id, name);
        return result.status === "error" ? result.error.code : null;
      }}
      onDuplicate={(id) => void run(commands.profileDuplicate(id))}
      onExport={(id) =>
        void run(commands.profileExport(id)).then((r) => {
          if (r?.data) setNotice({ tone: "info", content: t("profiles.exported", { path: r.data }) });
        })
      }
      onDelete={(id) => void run(commands.profileDelete(id))}
      onImport={() =>
        void run(commands.profileImport()).then((r) => {
          if (r?.data) imported(r.data);
        })
      }
      onImportFile={(file) => {
        if (file.size > MAX_IMPORT_BYTES) {
          fail({ code: "IMPORT_TOO_LARGE", message: "" });
          return;
        }
        void file
          .text()
          .then((text) => run(commands.profileImportText(text)))
          .then((r) => {
            if (r) imported(r.data);
          });
      }}
      onRecordHotkey={recording.recordProfile}
      onClearHotkey={recording.clearProfile}
      onCancelRecord={recording.cancel}
      onReplaceHotkey={recording.replace}
      onKeepHotkey={recording.keep}
    />
  );
}

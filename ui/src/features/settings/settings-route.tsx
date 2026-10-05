import { useState } from "react";

import { useTranslation } from "react-i18next";

import { commands } from "../../ipc";

import { applyCapabilities, applyUpdate, useAppStore } from "../../store/app-store";

import { themeLabel } from "../themes/themes-route";
import { SettingsScreen } from "./settings-screen";

import type { Settings } from "../../ipc";
import type { UpdateState } from "./settings-screen";
import type { ReactElement } from "react";

interface SettingsRouteProps {
  settings: Settings;
  onOpenThemes: () => void;
  onOpenHotkeys: () => void;
  onBack: () => void;
  onMinimize: () => void;
  onClose: () => void;
}

/** Tela 08 connected to Rust: each change is saved at once; Rust applies autostart and the theme. */
export function SettingsRoute({ settings, ...nav }: SettingsRouteProps): ReactElement {
  const { t } = useTranslation();
  const version = useAppStore((s) => s.version);
  const capabilities = useAppStore((s) => s.capabilities);
  const [error, setError] = useState<string | null>(null);
  const [measuring, setMeasuring] = useState(false);
  const found = useAppStore((s) => s.update);
  const [update, setUpdate] = useState<UpdateState>({ kind: "idle" });
  const shownUpdate: UpdateState =
    found && update.kind !== "installing" ? { kind: "available", version: found } : update;

  const save = (next: Settings) => {
    setError(null);
    void commands.settingsUpdate(next).then((r) => {
      if (r.status === "error") setError(r.error.code);
    });
  };

  return (
    <SettingsScreen
      settings={settings}
      version={version}
      capabilities={capabilities}
      themeLabel={themeLabel(settings, t)}
      measuring={measuring}
      errorCode={error}
      onChange={save}
      onLocale={(locale) => save({ ...settings, locale })}
      onSupport={() =>
        void commands.appOpenSupport().then((r) => {
          if (r.status === "error") setError(r.error.code);
        })
      }
      onMeasure={() => {
        setMeasuring(true);
        setError(null);
        void commands.engineRunBenchmark().then((r) => {
          setMeasuring(false);
          if (r.status === "ok") applyCapabilities(r.data);
          else setError("BENCHMARK_FAILED");
        });
      }}
      update={shownUpdate}
      onCheckUpdate={() => {
        setUpdate({ kind: "checking" });
        void commands.updateCheck().then((r) => {
          if (r.status === "error") setUpdate({ kind: "failed" });
          else if (r.data) {
            applyUpdate(r.data.version);
            setUpdate({ kind: "idle" });
          } else setUpdate({ kind: "latest" });
        });
      }}
      onInstallUpdate={() => {
        setUpdate({ kind: "installing" });
        void commands.updateInstall().then((r) => {
          if (r.status === "error") {
            setUpdate({ kind: "failed" });
            setError(r.error.code);
          }
        });
      }}
      {...nav}
    />
  );
}

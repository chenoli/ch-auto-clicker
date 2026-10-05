import { useState } from "react";

import { useTranslation } from "react-i18next";

import { Segmented } from "../../components/segmented";
import { Stepper } from "../../components/stepper";
import { Toggle } from "../../components/toggle";

import { DEFAULT_LOCALE, LOCALES, isLocale } from "../../i18n";

import { TitleBar } from "../main/title-bar";
import { SubScreenHeader } from "../shell/sub-screen-header";

import type { Locale } from "../../i18n";
import type { Capabilities, CloseAction, Settings } from "../../ipc";
import type { ReactElement, ReactNode } from "react";

export const COUNTDOWN_MAX = 10;

export type UpdateState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "latest" }
  | { kind: "available"; version: string }
  | { kind: "installing" }
  | { kind: "failed" };

function UpdateAction({
  update,
  onCheck,
  onInstall,
}: {
  update: UpdateState;
  onCheck?: (() => void) | undefined;
  onInstall?: (() => void) | undefined;
}) {
  const { t } = useTranslation();
  if (update.kind === "available") {
    return (
      <button type="button" className="link" onClick={onInstall}>
        {t("update.install", { version: update.version })}
      </button>
    );
  }
  if (update.kind === "checking" || update.kind === "installing") {
    return <span>{t(`update.${update.kind}`)}</span>;
  }
  return (
    <button type="button" className="footer-action" onClick={onCheck}>
      {update.kind === "idle" ? t("update.check") : t(`update.${update.kind}`)}
    </button>
  );
}

export interface SettingsScreenProps {
  settings: Settings;
  version: string;
  capabilities: Capabilities | null;
  themeLabel: string;
  measuring?: boolean;
  errorCode?: string | null;
  onChange: (next: Settings) => void;
  onLocale: (locale: Locale) => void;
  onOpenThemes: () => void;
  onOpenHotkeys: () => void;
  onSupport: () => void;
  onMeasure: () => void;
  update?: UpdateState;
  onCheckUpdate?: () => void;
  onInstallUpdate?: () => void;
  onBack: () => void;
  onMinimize: () => void;
  onClose: () => void;
}

function ToggleRow({
  title,
  hint,
  checked,
  onChange,
}: {
  title: string;
  hint: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div className="pref-row">
      <div className="flex flex-1 flex-col gap-0.75">
        <span className="text-13 font-semibold text-fg">{title}</span>
        <span className="text-11 text-fg-3">{hint}</span>
      </div>
      <Toggle label={title} checked={checked} onChange={onChange} />
    </div>
  );
}

function LinkRow({ label, value, onClick }: { label: string; value?: ReactNode; onClick: () => void }) {
  return (
    <button type="button" className="link-row" onClick={onClick}>
      {label}
      <span className="text-12 font-medium text-fg-2">
        {value ? <>{value} ›</> : <span className="text-fg-3">›</span>}
      </span>
    </button>
  );
}

function Advanced({
  capabilities,
  measuring,
  onMeasure,
}: {
  capabilities: Capabilities | null;
  measuring: boolean;
  onMeasure: () => void;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  return (
    <div className="border-t border-divider">
      <button type="button" className="link-row" aria-expanded={open} onClick={() => setOpen((o) => !o)}>
        {t("settings.advanced")}
        <span className="text-12 text-fg-3">{open ? "▴" : "▾"}</span>
      </button>
      {open && (
        <div className="flex flex-col gap-2.5 pb-4">
          <div className="flex items-baseline justify-between text-12">
            <span className="text-fg-2">{t("settings.minReliable")}</span>
            <span className="font-semibold text-fg tabular-nums">
              {capabilities?.measured
                ? `${capabilities.minReliableIntervalMs} ms`
                : t("settings.notMeasured", { ms: capabilities?.minReliableIntervalMs ?? 4 })}
            </span>
          </div>
          {capabilities?.measured && capabilities.results.length > 0 && (
            <table className="bench-table">
              <thead>
                <tr>
                  <th>{t("settings.benchInterval")}</th>
                  <th>p99</th>
                  <th>{t("settings.benchMissed")}</th>
                </tr>
              </thead>
              <tbody>
                {capabilities.results.map((r) => (
                  <tr key={r.intervalMs} data-reliable={r.reliable}>
                    <td>{r.intervalMs} ms</td>
                    <td>{Math.round(r.p99Us ?? 0)} µs</td>
                    <td>{r.missed}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          <div className="flex items-center justify-between gap-3">
            <span className="text-11 text-fg-3">{t("settings.measureHint")}</span>
            <button type="button" className="text-button flex-none" disabled={measuring} onClick={onMeasure}>
              {measuring ? t("settings.measuring") : t("settings.measure")}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export function SettingsScreen(props: SettingsScreenProps): ReactElement {
  const { t } = useTranslation();
  const { settings, onChange } = props;
  const set = (patch: Partial<Settings>) => onChange({ ...settings, ...patch });
  return (
    <div className="flex h-full flex-col bg-bg">
      <TitleBar onMinimize={props.onMinimize} onClose={props.onClose} />
      <SubScreenHeader title={t("settings.title")} onBack={props.onBack} />
      <div className="min-h-0 flex-1 overflow-auto px-4">
        {props.errorCode && (
          <p className="m-0 pb-3 text-12 text-danger" role="alert">
            {t(`errors.${props.errorCode}`, { defaultValue: t("errors.generic", { code: props.errorCode }) })}
          </p>
        )}
        <div className="flex flex-col gap-2.5 border-t border-divider py-3.5">
          <span className="micro-label">{t("settings.language")}</span>
          <Segmented<Locale>
            label={t("settings.language")}
            value={isLocale(settings.locale) ? settings.locale : DEFAULT_LOCALE}
            onChange={props.onLocale}
            options={LOCALES.map((l) => ({ value: l, label: t(`languages.${l}`) }))}
          />
        </div>
        <ToggleRow
          title={t("settings.autostart")}
          hint={t("settings.autostartHint")}
          checked={settings.autostart}
          onChange={(autostart) => set({ autostart })}
        />
        <ToggleRow
          title={t("settings.tray")}
          hint={t("settings.trayHint")}
          checked={settings.minimizeToTray}
          onChange={(minimizeToTray) => set({ minimizeToTray })}
        />
        <ToggleRow
          title={t("settings.hud")}
          hint={t("settings.hudHint")}
          checked={settings.showHud}
          onChange={(showHud) => set({ showHud })}
        />
        <div className="pref-row">
          <div className="flex flex-1 flex-col gap-0.75">
            <span className="text-13 font-semibold text-fg">{t("settings.countdown")}</span>
            <span className="text-11 text-fg-3">{t("settings.countdownHint")}</span>
          </div>
          <Stepper
            value={`${settings.countdownSec} s`}
            decrementLabel={t("settings.countdownLess")}
            incrementLabel={t("settings.countdownMore")}
            canDecrement={settings.countdownSec > 0}
            canIncrement={settings.countdownSec < COUNTDOWN_MAX}
            onDecrement={() => set({ countdownSec: Math.max(0, settings.countdownSec - 1) })}
            onIncrement={() => set({ countdownSec: Math.min(COUNTDOWN_MAX, settings.countdownSec + 1) })}
          />
        </div>
        <div className="pref-row grid grid-cols-[1fr_150px]">
          <span className="text-13 font-semibold text-fg">{t("settings.onClose")}</span>
          <Segmented<CloseAction>
            label={t("settings.onClose")}
            variant="compact"
            value={settings.closeAction}
            onChange={(closeAction) => set({ closeAction })}
            options={[
              { value: "minimize", label: t("settings.closeMinimize") },
              { value: "quit", label: t("settings.closeQuit") },
            ]}
          />
        </div>
        <div className="border-t border-divider">
          <LinkRow label={t("hotkeys.title")} onClick={props.onOpenHotkeys} />
          <div className="border-t border-divider">
            <LinkRow label={t("settings.theme")} value={props.themeLabel} onClick={props.onOpenThemes} />
          </div>
        </div>
        <Advanced capabilities={props.capabilities} measuring={props.measuring ?? false} onMeasure={props.onMeasure} />
        <div className="anti-cheat" role="note">
          <span className="anti-cheat__icon" aria-hidden="true">
            !
          </span>
          <div className="flex flex-col gap-1.5">
            <span className="text-13 font-semibold text-fg">{t("antiCheat.title")}</span>
            <span className="text-12 leading-relaxed text-fg-2">{t("antiCheat.text")}</span>
          </div>
        </div>
        <div className="support-card">
          <div className="flex min-w-0 flex-1 flex-col gap-1">
            <span className="support-card__kicker">{t("support.kicker")}</span>
            <span className="font-display text-17 leading-display font-extrabold tracking-logo text-fg">
              {t("support.title")}
            </span>
            <span className="text-11 leading-body text-fg-3">{t("support.text")}</span>
          </div>
          <button type="button" className="support-card__button" onClick={props.onSupport}>
            {t("support.action")}
          </button>
        </div>
      </div>
      <div className="flex h-9 flex-none items-center justify-between border-t border-divider px-4 text-11 text-fg-3">
        <span>{t("settings.footer", { version: props.version || "0.1.0" })}</span>
        {props.update && (
          <UpdateAction update={props.update} onCheck={props.onCheckUpdate} onInstall={props.onInstallUpdate} />
        )}
      </div>
    </div>
  );
}

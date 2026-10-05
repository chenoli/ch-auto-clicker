import { useState } from "react";

import { useTranslation } from "react-i18next";

import { HudView } from "../../features/hud/hud-view";
import { SettingsScreen } from "../../features/settings/settings-screen";

import { WindowFrame } from "./window-frame";

import type { Capabilities, Settings } from "../../ipc";
import type { ReactElement } from "react";

const noop = () => {};

const SETTINGS = {
  schemaVersion: 1,
  locale: "pt-BR",
  theme: "grafite",
  followSystemTheme: false,
  autostart: false,
  minimizeToTray: true,
  showHud: true,
  closeAction: "minimize",
  countdownSec: 3,
  inputBackend: "sendinput",
  activeProfileId: null,
  profileOrder: [],
  hotkeys: {},
  hud: { x: null, y: null },
  window: { x: null, y: null },
  antiCheatAcknowledged: false,
  hudHintsShown: 0,
} as unknown as Settings;

const CAPABILITIES = { minReliableIntervalMs: 4, measured: false, results: [] } as unknown as Capabilities;

export function SettingsScreenDemo(): ReactElement {
  const { i18n } = useTranslation();
  const [settings, setSettings] = useState({ ...SETTINGS, locale: i18n.language } as Settings);
  return (
    <WindowFrame width={420} height={640}>
      {() => (
        <SettingsScreen
          settings={settings}
          version="0.1.0"
          capabilities={CAPABILITIES}
          themeLabel="Grafite"
          onChange={setSettings}
          onLocale={(locale) => void i18n.changeLanguage(locale)}
          onOpenThemes={noop}
          onOpenHotkeys={noop}
          onSupport={noop}
          onMeasure={noop}
          onBack={noop}
          onMinimize={noop}
          onClose={noop}
        />
      )}
    </WindowFrame>
  );
}

export function HudStatesDemo(): ReactElement {
  const { i18n } = useTranslation();
  const common = {
    locale: i18n.language,
    profileName: "Coleta rápida",
    clicks: 1284,
    startStop: "F6",
    pauseResume: "F7",
  };
  return (
    <div className="hud-demo">
      <HudView {...common} state="countdown" countdownRemainingMs={2400} remainingMs={null} />
      <HudView {...common} state="running" countdownRemainingMs={null} remainingMs={272_000} />
      <HudView {...common} state="paused" countdownRemainingMs={null} remainingMs={272_000} />
      <HudView {...common} state="running" countdownRemainingMs={null} remainingMs={272_000} hover showHint />
    </div>
  );
}

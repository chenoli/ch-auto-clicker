import {
  HotkeysScreenDemo,
  HotkeysStatesDemo,
  MainIdleScreen,
  MainScanScreen,
  MainStatesScreen,
  MainWiggleScreen,
  VariationScreen,
} from "./main-window-screens";
import { SCREEN_META } from "./manifest";
import { AreaOverlayScreenDemo, OverlayScreenDemo, PointsScreenDemo } from "./points-screens";
import { ProfilesEmptyDemo, ProfilesScreenDemo, ThemesScreenDemo } from "./profiles-screens";
import { HudStatesDemo, SettingsScreenDemo } from "./settings-screens";
import { TokensScreen } from "./tokens-screen";
import { TrayMenuDemo } from "./tray-screens";

import type { ScreenMeta } from "./manifest";
import type { ReactNode } from "react";

export interface GalleryScreen extends ScreenMeta {
  render: () => ReactNode;
}

const RENDERERS: Record<string, () => ReactNode> = {
  "main-idle": () => <MainIdleScreen />,
  "main-wiggle": () => <MainWiggleScreen />,
  "main-scan": () => <MainScanScreen />,
  "main-states": () => <MainStatesScreen />,
  points: () => <PointsScreenDemo />,
  variation: () => <VariationScreen />,
  overlay: () => <OverlayScreenDemo />,
  "area-overlay": () => <AreaOverlayScreenDemo />,
  profiles: () => <ProfilesScreenDemo />,
  "profiles-empty": () => <ProfilesEmptyDemo />,
  hotkeys: () => <HotkeysScreenDemo />,
  "hotkeys-states": () => <HotkeysStatesDemo />,
  settings: () => <SettingsScreenDemo />,
  hud: () => <HudStatesDemo />,
  themes: () => <ThemesScreenDemo />,
  "tray-menu": () => <TrayMenuDemo />,
  tokens: () => <TokensScreen />,
};

export const SCREENS: GalleryScreen[] = SCREEN_META.map((meta) => {
  const render = RENDERERS[meta.id];
  if (!render) throw new Error(`gallery screen without renderer: ${meta.id}`);
  return { ...meta, render };
});

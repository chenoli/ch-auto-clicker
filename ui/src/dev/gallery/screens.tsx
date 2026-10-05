import {
  HotkeysScreenDemo,
  HotkeysStatesDemo,
  MainIdleScreen,
  MainStatesScreen,
  MainWiggleScreen,
  VariationScreen,
} from "./main-window-screens";
import { SCREEN_META } from "./manifest";
import { OverlayScreenDemo, PointsScreenDemo } from "./points-screens";
import { TokensScreen } from "./tokens-screen";

import type { ScreenMeta } from "./manifest";
import type { ReactNode } from "react";

export interface GalleryScreen extends ScreenMeta {
  render: () => ReactNode;
}

const RENDERERS: Record<string, () => ReactNode> = {
  "main-idle": () => <MainIdleScreen />,
  "main-wiggle": () => <MainWiggleScreen />,
  "main-states": () => <MainStatesScreen />,
  points: () => <PointsScreenDemo />,
  variation: () => <VariationScreen />,
  overlay: () => <OverlayScreenDemo />,
  hotkeys: () => <HotkeysScreenDemo />,
  "hotkeys-states": () => <HotkeysStatesDemo />,
  tokens: () => <TokensScreen />,
};

export const SCREENS: GalleryScreen[] = SCREEN_META.map((meta) => {
  const render = RENDERERS[meta.id];
  if (!render) throw new Error(`gallery screen without renderer: ${meta.id}`);
  return { ...meta, render };
});

import { useCallback, useEffect, useState } from "react";

import { commands, inTauri } from "../../ipc";

import { editProfile } from "../../store/profile-sync";

import { hotkeyLabel } from "../hotkeys/format-hotkey";
import { PointsScreen } from "./points-screen";

import type { Profile, ScreenLayout, ScreenPoint } from "../../ipc";
import type { ReactElement } from "react";

interface PointsRouteProps {
  profile: Profile;
  captureHotkey: Parameters<typeof hotkeyLabel>[0];
  onBack: () => void;
  onMinimize: () => void;
  onClose: () => void;
}

const EMPTY_LAYOUT: ScreenLayout = { monitors: [], fingerprint: "" };

/** Where "+ Manual" puts a new point: next to the last one, or the middle of the primary screen. */
export function manualPoint(points: readonly ScreenPoint[], layout: ScreenLayout, id: string): ScreenPoint {
  const last = points.at(-1);
  if (last) return { id, x: last.x + 40, y: last.y, delayMs: 0 };
  const primary = layout.monitors.find((m) => m.x === 0 && m.y === 0) ?? layout.monitors[0];
  if (!primary) return { id, x: 960, y: 540, delayMs: 0 };
  return {
    id,
    x: primary.x + Math.round(primary.width / 2),
    y: primary.y + Math.round(primary.height / 2),
    delayMs: 0,
  };
}

/** Points editor connected to Rust: screen layout, capture overlay and profile edits. */
export function PointsRoute({ profile, captureHotkey, onBack, onMinimize, onClose }: PointsRouteProps): ReactElement {
  const [layout, setLayout] = useState<ScreenLayout>(EMPTY_LAYOUT);
  const [error, setError] = useState<string | null>(null);
  const click = profile.modes.click;

  const refreshLayout = useCallback(() => {
    if (inTauri) void commands.screenLayout().then(setLayout);
  }, []);

  useEffect(() => {
    refreshLayout();
    window.addEventListener("focus", refreshLayout);
    return () => window.removeEventListener("focus", refreshLayout);
  }, [refreshLayout]);

  const setPoints = (points: ScreenPoint[]) => {
    setError(null);
    // Hand-edited coordinates belong to the arrangement the user is looking at now.
    const fingerprint = layout.fingerprint || click.layoutFingerprint;
    editProfile(profile.id, (p) => ({
      ...p,
      modes: { ...p.modes, click: { ...p.modes.click, points, layoutFingerprint: fingerprint } },
    }));
  };

  const capture = async () => {
    setError(null);
    const result = await commands.overlayPickPoints();
    if (result.status === "error") setError(result.error.code);
    refreshLayout();
  };

  return (
    <PointsScreen
      points={click.points}
      monitors={layout.monitors}
      layoutChanged={
        click.points.length > 0 &&
        layout.fingerprint !== "" &&
        click.layoutFingerprint !== null &&
        click.layoutFingerprint !== layout.fingerprint
      }
      captureShortcut={hotkeyLabel(captureHotkey)}
      errorCode={error}
      onChange={setPoints}
      onCapture={() => void capture()}
      onAddManual={() => setPoints([...click.points, manualPoint(click.points, layout, crypto.randomUUID())])}
      onBack={onBack}
      onMinimize={onMinimize}
      onClose={onClose}
    />
  );
}

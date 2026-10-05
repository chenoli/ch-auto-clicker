import { useEffect, useState } from "react";

import { getCurrentWindow } from "@tauri-apps/api/window";

import { commands } from "../../ipc";

import { selectActiveProfile, useAppStore } from "../../store/app-store";
import { connectStore } from "../../store/connect";

import { ConnectFailure } from "../shell/connect-failure";
import { VariationEditorView } from "./variation-editor-view";

import type { Profile } from "../../ipc";
import type { ReactElement } from "react";

const DEFAULT_MIN_RELIABLE_MS = 4;

export function VariationWindow(): ReactElement | null {
  const ready = useAppStore((s) => s.ready);
  const profile = useAppStore(selectActiveProfile);
  const settings = useAppStore((s) => s.settings);
  const capabilities = useAppStore((s) => s.capabilities);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const [connectError, setConnectError] = useState<string | null>(null);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    connectStore()
      .then((d) => (dispose = d))
      .catch((e: unknown) => setConnectError(String(e)));
    return () => dispose?.();
  }, []);

  if (connectError) return <ConnectFailure detail={connectError} />;

  if (!ready || !profile || !settings) return null;

  const close = () => void getCurrentWindow().close();

  const save = async (mutate: (p: Profile) => Profile) => {
    setBusy(true);
    const result = await commands.profileUpdate(mutate(profile), profile.updatedAt);
    setBusy(false);
    if (result.status === "ok") close();
    else setError(result.error.code);
  };

  return (
    <VariationEditorView
      initialBlocks={profile.click.variation.blocksMs}
      double={profile.click.kind === "double"}
      locale={settings.locale}
      minReliableMs={capabilities?.minReliableIntervalMs ?? DEFAULT_MIN_RELIABLE_MS}
      error={error}
      busy={busy}
      onApply={(blocksMs) =>
        void save((p) => ({ ...p, click: { ...p.click, variation: { enabled: true, blocksMs } } }))
      }
      onDisable={() =>
        void save((p) => ({ ...p, click: { ...p.click, variation: { ...p.click.variation, enabled: false } } }))
      }
      onClose={close}
      onMinimize={() => void getCurrentWindow().minimize()}
    />
  );
}

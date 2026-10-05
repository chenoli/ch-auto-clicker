import { useTranslation } from "react-i18next";

import { Callout } from "../../components/callout";

import type { ReactElement } from "react";

/** Shown instead of a blank window when the UI cannot reach the Rust side. */
export function ConnectFailure({ detail }: { detail: string }): ReactElement {
  const { t } = useTranslation();
  return (
    <div className="flex h-full flex-col justify-center gap-3 bg-bg p-6" data-tauri-drag-region>
      <Callout tone="danger">
        <strong>{t("errors.connect")}</strong>
        <br />
        <span className="text-11 break-all text-fg-3">{detail}</span>
      </Callout>
    </div>
  );
}

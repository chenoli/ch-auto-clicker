import { useEffect, useRef } from "react";

import { useTranslation } from "react-i18next";

import { Button } from "../../components/button";

import type { ReactElement } from "react";

interface AntiCheatDialogProps {
  onAccept: () => void;
  onCancel: () => void;
}

export function AntiCheatDialog({ onAccept, onCancel }: AntiCheatDialogProps): ReactElement {
  const { t } = useTranslation();
  const acceptRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    acceptRef.current?.focus();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onCancel();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);
  return (
    <div className="dialog-backdrop">
      <div className="dialog" role="alertdialog" aria-modal="true" aria-labelledby="anti-cheat-title">
        <span className="anti-cheat__icon" aria-hidden="true">
          !
        </span>
        <h2 id="anti-cheat-title" className="m-0 font-display text-20 font-extrabold tracking-logo text-fg">
          {t("antiCheat.title")}
        </h2>
        <p className="m-0 text-13 leading-relaxed text-fg-2">{t("antiCheat.text")}</p>
        <div className="flex justify-end gap-2 pt-1">
          <Button variant="text" onClick={onCancel}>
            {t("common.cancel")}
          </Button>
          <Button ref={acceptRef} variant="primary" className="h-9 px-3.5" onClick={onAccept}>
            {t("antiCheat.accept")}
          </Button>
        </div>
      </div>
    </div>
  );
}

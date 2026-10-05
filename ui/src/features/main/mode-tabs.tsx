import { useTranslation } from "react-i18next";

import { Segmented } from "../../components/segmented";

import type { Mode } from "../../ipc";
import type { ReactElement } from "react";

export function ModeTabs({ mode, onChange }: { mode: Mode; onChange: (mode: Mode) => void }): ReactElement {
  const { t } = useTranslation();
  return (
    <div className="flex-none px-4">
      <Segmented
        label={t("header.profile")}
        variant="tabs"
        value={mode}
        onChange={onChange}
        options={[
          { value: "click", label: t("modes.click"), index: "01" },
          { value: "wiggle", label: t("modes.wiggle"), index: "02" },
          { value: "scan", label: t("modes.scan"), index: "03" },
        ]}
      />
    </div>
  );
}

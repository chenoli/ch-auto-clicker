import type { ReactElement, ReactNode } from "react";

export function Callout({ children, tone = "warn" }: { children: ReactNode; tone?: "warn" | "danger" }): ReactElement {
  return (
    <div className="callout" data-tone={tone} role={tone === "danger" ? "alert" : "note"}>
      <span className="callout__dot" />
      <div>{children}</div>
    </div>
  );
}

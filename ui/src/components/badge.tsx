import type { ReactElement, ReactNode } from "react";

export function Badge({
  children,
  tone = "accent",
}: {
  children: ReactNode;
  tone?: "accent" | "neutral";
}): ReactElement {
  return <span className={`badge ${tone === "neutral" ? "badge--neutral" : ""}`}>{children}</span>;
}

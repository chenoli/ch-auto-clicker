import type { ReactElement, ReactNode } from "react";

interface KbdProps {
  children: ReactNode;
  size?: "default" | "large";
  /** Struck-through and dashed: the shortcut could not be registered. */
  broken?: boolean;
}

export function Kbd({ children, size = "default", broken = false }: KbdProps): ReactElement {
  return (
    <kbd className={`kbd ${size === "large" ? "kbd--large" : ""}`} data-broken={broken}>
      {children}
    </kbd>
  );
}

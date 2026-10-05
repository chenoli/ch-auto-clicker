import type { ReactElement } from "react";

type LogoSize = "title" | "display";

const SIZE_CLASS: Record<LogoSize, string> = {
  title: "text-14",
  display: "text-28",
};

export function Logo({ size = "title" }: { size?: LogoSize }): ReactElement {
  return (
    <span className={`font-display leading-none font-extrabold tracking-logo text-fg ${SIZE_CLASS[size]}`}>
      CH<span className="text-accent">.</span>
    </span>
  );
}

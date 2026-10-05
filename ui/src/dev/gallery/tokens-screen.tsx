import { THEMES } from "../../styles/tokens";

import type { ReactElement } from "react";

const SWATCHES = [
  "bg",
  "bg-2",
  "bg-3",
  "fg",
  "fg-2",
  "fg-3",
  "border",
  "accent",
  "accent-b",
  "warn",
  "danger",
  "on-accent",
];
const DERIVED = [
  "divider",
  "border-strong",
  "seg-active",
  "tint-accent",
  "tint-warn",
  "tint-danger",
  "edge-accent",
  "edge-warn",
  "edge-danger",
];

// Each block sets its own data-theme, which also proves derived tokens recompute in nested themes.
export function TokensScreen(): ReactElement {
  return (
    <div className="grid grid-cols-2 gap-3">
      {THEMES.map((theme) => (
        <section
          key={theme.id}
          data-theme={theme.id}
          className="flex flex-col gap-2 rounded-card border border-border bg-bg p-3 text-fg"
        >
          <header className="flex items-baseline justify-between">
            <span className="font-display text-17 font-extrabold tracking-logo">{theme.name}</span>
            <span className="text-9 font-bold tracking-label text-fg-3 uppercase">{theme.tag}</span>
          </header>
          <div className="grid grid-cols-12 gap-1">
            {SWATCHES.map((token) => (
              <div
                key={token}
                title={`--${token}`}
                className="h-5 rounded-seg-item border border-border"
                style={{ background: `var(--${token})` }}
              />
            ))}
          </div>
          <div className="grid grid-cols-9 gap-1">
            {DERIVED.map((token) => (
              <div
                key={token}
                title={`--${token}`}
                className="h-3 rounded-seg-item"
                style={{ background: `var(--${token})` }}
              />
            ))}
          </div>
          <p className="m-0 text-11 text-fg-2">
            Space Grotesk 500 · <span className="font-semibold text-accent tabular-nums">1.284 cliques</span>
          </p>
        </section>
      ))}
    </div>
  );
}

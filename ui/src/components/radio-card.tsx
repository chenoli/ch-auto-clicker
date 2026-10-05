import type { ReactElement, ReactNode } from "react";

interface RadioCardProps {
  checked: boolean;
  onSelect: () => void;
  label: string;
  /** Extra content on the right (e.g. the "4 pontos ›" link). */
  trailing?: ReactNode;
}

export function RadioCard({ checked, onSelect, label, trailing }: RadioCardProps): ReactElement {
  return (
    <div
      className="radio-card"
      role="radio"
      aria-checked={checked}
      tabIndex={0}
      onClick={onSelect}
      onKeyDown={(e) => {
        if (e.key === " " || e.key === "Enter") {
          e.preventDefault();
          onSelect();
        }
      }}
    >
      <span className="radio-card__ring">
        <span className="radio-card__dot" />
      </span>
      <span className="flex-1 text-13 font-medium text-fg">{label}</span>
      {trailing}
    </div>
  );
}

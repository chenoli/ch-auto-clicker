import type { ReactElement, ReactNode } from "react";

export interface SegmentedOption<T extends string> {
  value: T;
  label: ReactNode;
  index?: string;
}

interface SegmentedProps<T extends string> {
  options: readonly SegmentedOption<T>[];
  value: T;
  onChange: (value: T) => void;
  label: string;
  variant?: "default" | "compact" | "tabs";
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
  label,
  variant = "default",
}: SegmentedProps<T>): ReactElement {
  const variantClass = variant === "tabs" ? "mode-tabs" : variant === "compact" ? "segmented--compact" : "";
  return (
    <div role="radiogroup" aria-label={label} className={`segmented ${variantClass}`}>
      {options.map((option) => {
        const active = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={active}
            data-active={active}
            className="segmented__item"
            onClick={() => onChange(option.value)}
          >
            {option.index && <span className="mode-tabs__index">{option.index}</span>}
            <span>{option.label}</span>
          </button>
        );
      })}
    </div>
  );
}

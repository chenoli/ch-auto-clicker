import type { ReactElement } from "react";

interface BigNumberFieldProps {
  value: string;
  onChange: (raw: string) => void;
  onCommit?: () => void;
  unit: string;
  label: string;
  inputMode?: "numeric" | "decimal";
}

export function BigNumberField({
  value,
  onChange,
  onCommit,
  unit,
  label,
  inputMode = "numeric",
}: BigNumberFieldProps): ReactElement {
  return (
    <label className="big-field">
      <input
        className="big-field__input"
        aria-label={label}
        inputMode={inputMode}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onBlur={onCommit}
        onKeyDown={(e) => {
          if (e.key === "Enter") onCommit?.();
        }}
      />
      <span className="big-field__unit">{unit}</span>
    </label>
  );
}

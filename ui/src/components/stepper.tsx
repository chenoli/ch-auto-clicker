import type { ReactElement } from "react";

interface StepperProps {
  value: string;
  onDecrement: () => void;
  onIncrement: () => void;
  decrementLabel: string;
  incrementLabel: string;
  canDecrement?: boolean;
  canIncrement?: boolean;
  /** Dimmed value, e.g. while the stop timer is off. */
  muted?: boolean;
}

export function Stepper({
  value,
  onDecrement,
  onIncrement,
  decrementLabel,
  incrementLabel,
  canDecrement = true,
  canIncrement = true,
  muted = false,
}: StepperProps): ReactElement {
  return (
    <div className="stepper" data-muted={muted}>
      <button
        type="button"
        className="stepper__button"
        aria-label={decrementLabel}
        disabled={!canDecrement}
        onClick={onDecrement}
      >
        −
      </button>
      <span className="stepper__value" aria-live="polite">
        {value}
      </span>
      <button
        type="button"
        className="stepper__button"
        aria-label={incrementLabel}
        disabled={!canIncrement}
        onClick={onIncrement}
      >
        +
      </button>
    </div>
  );
}

import { forwardRef } from "react";
import { ToggleButton, type ToggleButtonProps } from "./ToggleButton";

export interface ReactionToggleProps extends Omit<ToggleButtonProps, "children" | "size"> {
  emoji: string;
  count?: number;
  label: string;
  pressed: boolean;
  loading?: boolean;
}

/** Compact timeline reaction; its 24px geometry is intentionally domain-specific. */
export const ReactionToggle = forwardRef<HTMLButtonElement, ReactionToggleProps>(
  function ReactionToggle(
    {
      className,
      count = 0,
      disabled,
      emoji,
      label,
      loading = false,
      pressed,
      type = "button",
      ...props
    },
    ref,
  ) {
    return (
      <ToggleButton
        ref={ref}
        aria-label={label}
        pressed={pressed}
        size="xs"
        loading={loading}
        disabled={disabled}
        className={className}
        type={type}
        {...props}
      >
        <span aria-hidden="true">{emoji}</span>
        {count > 0 ? <span className="tabular-nums">{count}</span> : null}
      </ToggleButton>
    );
  },
);

import { forwardRef } from "react";
import { cn } from "@/lib/utils";
import { Button, type ButtonProps } from "./Button";

export interface ToggleButtonProps extends ButtonProps {
  pressed: boolean;
}

/** A labelled binary action with an explicit pressed state. */
export const ToggleButton = forwardRef<HTMLButtonElement, ToggleButtonProps>(function ToggleButton(
  { className, pressed, variant = "secondary", ...props },
  ref,
) {
  return (
    <Button
      ref={ref}
      aria-pressed={pressed}
      data-state={pressed ? "on" : "off"}
      variant={variant}
      className={cn(pressed && "border-gold/40 bg-gold/10 text-text", className)}
      {...props}
    />
  );
});

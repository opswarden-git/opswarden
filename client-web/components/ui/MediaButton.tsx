import { forwardRef } from "react";
import { cn } from "@/lib/utils";
import { Button, type ButtonProps } from "./Button";

export interface MediaButtonProps extends Omit<ButtonProps, "fullWidth" | "size" | "variant"> {
  label: string;
}

/** A consistent interactive thumbnail for media pickers. */
export const MediaButton = forwardRef<HTMLButtonElement, MediaButtonProps>(function MediaButton(
  { className, label, ...props },
  ref,
) {
  return (
    <Button
      ref={ref}
      aria-label={label}
      variant="secondary"
      fullWidth
      className={cn("aspect-[4/3] h-auto overflow-hidden p-0", className)}
      {...props}
    />
  );
});

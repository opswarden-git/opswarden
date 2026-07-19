import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export type ButtonVariant = "primary" | "secondary" | "danger" | "ghost";
export type ButtonSize = "xs" | "sm" | "md" | "lg";
export type IconButtonTone = "neutral" | "accent" | "danger";

const variantClasses: Record<ButtonVariant, string> = {
  primary: "bg-gold text-gold-ink hover:bg-gold-hover",
  secondary: "border-border bg-panel text-text hover:bg-panel-2 border",
  danger:
    "border-danger bg-danger text-danger-ink hover:border-danger-hover hover:bg-danger-hover border",
  ghost: "text-muted hover:bg-panel-2 hover:text-text",
};

const sizeClasses: Record<ButtonSize, string> = {
  xs: "h-6 gap-1 px-1.5 text-xs",
  sm: "h-8 gap-1.5 px-3 text-xs",
  md: "h-9 gap-2 px-3.5 text-sm",
  lg: "h-10 gap-2 px-4 text-sm",
};

const iconSizeClasses: Record<ButtonSize, string> = {
  xs: "h-6 w-6 p-0",
  sm: "h-8 w-8 p-0",
  md: "h-9 w-9 p-0",
  lg: "h-10 w-10 p-0",
};

const iconToneClasses: Record<IconButtonTone, string> = {
  neutral: "",
  accent: "text-gold hover:text-gold",
  danger:
    "text-sev-critical hover:bg-sev-critical/10 hover:text-sev-critical focus-visible:ring-sev-critical/40",
};

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  loading?: boolean;
  fullWidth?: boolean;
}

interface ButtonClassNamesOptions {
  className?: string;
  fullWidth?: boolean;
  size?: ButtonSize;
  variant?: ButtonVariant;
}

/** Shared visual contract for button-shaped links and the Button primitive. */
export function buttonClassNames({
  className,
  fullWidth = false,
  size = "md",
  variant = "secondary",
}: ButtonClassNamesOptions = {}) {
  return cn(
    "focus-visible:ring-gold/50 focus-visible:ring-offset-bg inline-flex shrink-0 items-center justify-center rounded-md font-medium whitespace-nowrap transition-colors focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50",
    variantClasses[variant],
    sizeClasses[size],
    fullWidth && "w-full",
    className,
  );
}

/**
 * The single button contract for product actions.
 *
 * Screens may control placement and width through `className`, while the
 * variant and size props own the visual treatment and internal geometry.
 */
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  {
    children,
    className,
    disabled,
    fullWidth = false,
    loading = false,
    size = "md",
    type = "button",
    variant = "secondary",
    ...props
  },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      className={buttonClassNames({ className, fullWidth, size, variant })}
      {...props}
    >
      {loading ? (
        <span
          className="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
          aria-hidden="true"
        />
      ) : null}
      {children}
    </button>
  );
});

export interface IconButtonProps extends Omit<ButtonProps, "children" | "fullWidth"> {
  label: string;
  children: ReactNode;
  tone?: IconButtonTone;
}

/** A labelled square button for icon-only actions. */
export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton(
  { children, className, label, loading = false, size = "md", tone = "neutral", ...props },
  ref,
) {
  return (
    <Button
      ref={ref}
      aria-label={label}
      size={size}
      className={cn(iconSizeClasses[size], iconToneClasses[tone], className)}
      loading={loading}
      {...props}
    >
      {loading ? null : children}
    </Button>
  );
});

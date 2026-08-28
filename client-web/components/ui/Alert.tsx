import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/utils";

export type AlertTone = "info" | "success" | "warning" | "danger";

// Consumes semantic feedback tokens: text-feedback-success, text-feedback-warning, text-feedback-danger
const toneClasses: Record<AlertTone, string> = {
  info: "bg-st-ack text-white font-medium",
  success: "bg-feedback-success text-white font-medium",
  warning: "bg-feedback-warning text-white font-medium",
  danger: "bg-feedback-danger text-white font-medium",
};

export interface AlertProps extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
  tone?: AlertTone;
  title?: ReactNode;
}

/** A system message. Alerts describe state and never look like actions. */
export function Alert({ children, className, role, title, tone = "info", ...props }: AlertProps) {
  return (
    <div
      role={role ?? (tone === "danger" ? "alert" : "status")}
      className={cn(
        "rounded-md p-3 text-sm font-medium text-white shadow-sm",
        toneClasses[tone],
        className,
      )}
      {...props}
    >
      {title ? <div className="mb-1 font-semibold text-white">{title}</div> : null}
      <div>{children}</div>
    </div>
  );
}

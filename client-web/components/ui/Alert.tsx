import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/utils";

export type AlertTone = "info" | "success" | "warning" | "danger";

const toneClasses: Record<AlertTone, string> = {
  info: "border-st-ack/30 bg-st-ack/10 text-st-ack",
  success: "border-feedback-success/30 bg-feedback-success/10 text-feedback-success",
  warning: "border-feedback-warning/30 bg-feedback-warning/10 text-feedback-warning",
  danger: "border-feedback-danger/30 bg-feedback-danger/10 text-feedback-danger",
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
      className={cn("rounded-md border p-3 text-sm", toneClasses[tone], className)}
      {...props}
    >
      {title ? <div className="text-text mb-1 font-medium">{title}</div> : null}
      <div>{children}</div>
    </div>
  );
}

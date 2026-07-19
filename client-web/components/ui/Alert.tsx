import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/utils";

export type AlertTone = "info" | "success" | "warning" | "danger";

const toneClasses: Record<AlertTone, string> = {
  info: "border-st-ack/30 bg-st-ack/10 text-st-ack",
  success: "border-st-res/30 bg-st-res/10 text-st-res",
  warning: "border-sev-medium/30 bg-sev-medium/10 text-sev-medium",
  danger: "border-sev-critical/30 bg-sev-critical/10 text-sev-critical",
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

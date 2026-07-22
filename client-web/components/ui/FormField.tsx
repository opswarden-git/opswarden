"use client";

import React, { type ReactNode, useId } from "react";
import { cn } from "@/lib/utils";

export interface FormFieldProps {
  /** Visible label text */
  label: ReactNode;
  /** Optional hint/caption below the input */
  caption?: ReactNode;
  /** Error message, renders below the input and marks the field as invalid */
  error?: ReactNode;
  /** Whether the field is required */
  required?: boolean;
  /** Additional className on the root wrapper */
  className?: string;
  /** The input/select/textarea to wrap */
  children: ReactNode;
}

/**
 * Structural wrapper that ties a label, caption and error to a single form
 * control via stable ARIA ids. Does **not** own state, validation or mutations.
 *
 * The child control receives `id`, `aria-describedby` and `aria-invalid` via
 * `React.cloneElement`. If the child is a composite component, forward those
 * props to the underlying `<input>` / `<select>` / `<textarea>`.
 */
export function FormField({
  label,
  caption,
  error,
  required,
  className,
  children,
}: FormFieldProps) {
  const autoId = useId();
  const controlId = `ff-${autoId}`;
  const captionId = caption && !error ? `${controlId}-caption` : undefined;
  const errorId = error ? `${controlId}-error` : undefined;

  const describedBy = [captionId, errorId].filter(Boolean).join(" ") || undefined;

  const enhanced = React.isValidElement(children)
    ? React.cloneElement(children as React.ReactElement<Record<string, unknown>>, {
        id: controlId,
        "aria-describedby": describedBy,
        "aria-invalid": error ? true : undefined,
        "aria-required": required || undefined,
      })
    : children;

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <label htmlFor={controlId} className="text-text text-sm font-medium">
        {label}
        {required && (
          <span className="text-sev-critical ml-0.5" aria-hidden="true">
            *
          </span>
        )}
      </label>

      {enhanced}

      {caption && !error ? (
        <p id={captionId} className="text-muted text-xs">
          {caption}
        </p>
      ) : null}

      {error ? (
        <p id={errorId} className="text-sev-critical text-xs" role="alert">
          {error}
        </p>
      ) : null}
    </div>
  );
}

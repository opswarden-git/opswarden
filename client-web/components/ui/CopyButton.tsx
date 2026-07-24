"use client";

import { Check, Copy } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { IconButton, type IconButtonProps } from "./Button";
import { useTranslations } from "next-intl";

type CopyStatus = "idle" | "copied" | "error";

export interface CopyButtonProps extends Omit<IconButtonProps, "children" | "onClick" | "tone"> {
  value: string;
  copiedLabel?: string;
  errorLabel?: string;
  feedbackDuration?: number;
  onCopyError?: (error: unknown) => void;
}

/** Clipboard action with success/error feedback and an accessible announcement. */
export function CopyButton({
  copiedLabel,
  errorLabel,
  feedbackDuration = 2000,
  label,
  onCopyError,
  value,
  ...props
}: CopyButtonProps) {
  const t = useTranslations("Common");
  const [status, setStatus] = useState<CopyStatus>("idle");
  const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (resetTimer.current) clearTimeout(resetTimer.current);
    },
    [],
  );

  const resetLater = () => {
    if (resetTimer.current) clearTimeout(resetTimer.current);
    resetTimer.current = setTimeout(() => setStatus("idle"), feedbackDuration);
  };

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(value);
      setStatus("copied");
    } catch (error) {
      setStatus("error");
      onCopyError?.(error);
    }
    resetLater();
  };

  const feedback =
    status === "copied"
      ? (copiedLabel ?? t("copied"))
      : status === "error"
        ? (errorLabel ?? t("copyFailed"))
        : "";

  return (
    <span className="inline-flex">
      <IconButton
        {...props}
        label={feedback || label}
        onClick={copy}
        tone={status === "error" ? "danger" : status === "copied" ? "accent" : "neutral"}
      >
        {status === "copied" ? (
          <Check className="h-4 w-4" aria-hidden="true" />
        ) : (
          <Copy className="h-4 w-4" aria-hidden="true" />
        )}
      </IconButton>
      <span className="sr-only" aria-live="polite">
        {feedback}
      </span>
    </span>
  );
}

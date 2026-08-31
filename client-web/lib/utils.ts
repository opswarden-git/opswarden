import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** Formats a relative age (e.g. "2 hours ago", "in 5 minutes"). */
export function formatRelativeAge(value: string | number | Date, locale: string): string {
  const date = value instanceof Date ? value : new Date(value);
  const seconds = Math.round((date.getTime() - Date.now()) / 1000);
  const ranges: Array<[Intl.RelativeTimeFormatUnit, number]> = [
    ["day", 86_400],
    ["hour", 3_600],
    ["minute", 60],
  ];
  const formatter = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });

  for (const [unit, size] of ranges) {
    if (Math.abs(seconds) >= size) return formatter.format(Math.round(seconds / size), unit);
  }

  return formatter.format(seconds, "second");
}

/** Formats a timestamp into localized date & time string. */
export function formatDateTime(
  value: string | number | Date,
  locale: string,
  options: Intl.DateTimeFormatOptions = { dateStyle: "medium", timeStyle: "short" },
): string {
  const date = value instanceof Date ? value : new Date(value);
  if (isNaN(date.getTime())) return "—";
  return new Intl.DateTimeFormat(locale, options).format(date);
}

/** Formats a timestamp into localized date-only string. */
export function formatDateOnly(
  value: string | number | Date,
  locale: string,
  style: "short" | "medium" | "long" = "medium",
): string {
  return formatDateTime(value, locale, { dateStyle: style });
}

/** Formats a timestamp into localized time-only string. */
export function formatTimeOnly(
  value: string | number | Date,
  locale: string,
  style: "short" | "medium" = "short",
): string {
  return formatDateTime(value, locale, { timeStyle: style });
}

/** Formats a duration in milliseconds into human readable format (e.g., "120ms" or "2.4s"). */
export function formatDurationMs(ms: number): string {
  if (ms < 0) return "0ms";
  if (ms < 1000) return `${Math.round(ms)}ms`;
  const seconds = (ms / 1000).toFixed(1);
  return `${seconds.endsWith(".0") ? seconds.slice(0, -2) : seconds}s`;
}

/** Truncates a UUID or identifier to a short prefix for compact display. */
export function formatShortId(id: string | null | undefined, length = 8): string {
  if (!id) return "—";
  return id.length > length ? id.slice(0, length) : id;
}

/** Returns the value or a standardized fallback dash when blank/null/undefined. */
export function formatFallback(value: string | null | undefined, fallback = "—"): string {
  if (!value || value.trim().length === 0) return fallback;
  return value;
}

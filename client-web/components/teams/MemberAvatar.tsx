import { cn } from "@/lib/utils";

export function memberInitials(email: string): string {
  const local = email.split("@")[0] ?? email;
  const parts = local.split(/[._-]+/).filter(Boolean);
  const letters = parts.length >= 2 ? parts[0][0] + parts[1][0] : local.slice(0, 2);
  return letters.toUpperCase();
}

export function memberDisplayName(email: string): string {
  const local = email.split("@")[0] ?? email;
  return local
    .split(/[._-]+/)
    .filter(Boolean)
    .map((part) => `${part.charAt(0).toLocaleUpperCase()}${part.slice(1)}`)
    .join(" ");
}

export function MemberAvatar({ email, className }: { email: string; className?: string }) {
  return (
    <span
      className={cn(
        "surface-subtle text-muted border-border flex h-9 w-9 shrink-0 items-center justify-center rounded-full border text-xs font-semibold",
        className,
      )}
      aria-hidden="true"
    >
      {memberInitials(email)}
    </span>
  );
}

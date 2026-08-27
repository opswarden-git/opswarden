import { UserRound } from "lucide-react";
import type { TeamRole } from "@/lib/capabilities";
import { cn } from "@/lib/utils";
import { RoleIcon } from "./RoleChip";

export function memberDisplayName(email: string): string {
  const local = email.split("@")[0] ?? email;
  return local
    .split(/[._-]+/)
    .filter(Boolean)
    .map((part) => `${part.charAt(0).toLocaleUpperCase()}${part.slice(1)}`)
    .join(" ");
}

export function MemberAvatar({
  email,
  role,
  className,
}: {
  email: string;
  role?: TeamRole;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "text-gold flex h-9 w-9 shrink-0 items-center justify-center",
        className,
      )}
      title={email}
      aria-hidden="true"
    >
      {role ? (
        <RoleIcon role={role} className="h-2/3 w-2/3" />
      ) : (
        <UserRound
          className="text-gold h-2/3 w-2/3"
          strokeWidth={1.8}
          aria-hidden="true"
        />
      )}
    </span>
  );
}

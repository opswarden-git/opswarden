"use client";

import { useTranslations } from "next-intl";
import { MemberAvatar, memberDisplayName } from "@/components/teams/MemberAvatar";
import { RoleChip } from "@/components/teams/RoleChip";
import { Link } from "@/i18n/routing";
import type { TeamMember } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { cn } from "@/lib/utils";
import { useTeamOnline } from "@/lib/ws";
import { PaneSection } from "@/components/ui/PaneSection";
import { useAuthStore } from "@/store/auth";

export function TeamPresenceList({
  activePeerId,
  className,
  members,
  presentUserIds = [],
  teamId,
}: {
  activePeerId?: string;
  className?: string;
  members: TeamMember[];
  presentUserIds?: string[];
  teamId: string;
}) {
  const t = useTranslations("Teams");
  const tMessages = useTranslations("DirectMessages");
  const currentUserId = useAuthStore((state) => state.user?.id);
  const onlineIds = useTeamOnline(teamId);
  const activeIds = new Set([...onlineIds, ...presentUserIds]);
  const orderedMembers = [...members].sort((left, right) => {
    if (left.user_id === currentUserId) return -1;
    if (right.user_id === currentUserId) return 1;
    return 0;
  });

  return (
    <PaneSection className={className} title={t("members")} titleId="team-presence-title">
      <ul className="space-y-1 px-2">
        {orderedMembers.map((member) => {
          const self = member.user_id === currentUserId;
          const active = self || activeIds.has(member.user_id);
          const current = member.user_id === activePeerId;
          const itemClassName = cn(
            "flex min-w-0 items-center gap-2 rounded-md px-2 py-2 transition-[background-color,color,opacity]",
            current ? "bg-panel-2 text-text" : "text-muted",
            !self && "hover:bg-panel-2",
            !active && !current && "opacity-45 hover:opacity-100",
          );
          const content = (
            <>
              <span className="relative shrink-0">
                <MemberAvatar
                  email={member.email}
                  role={member.role}
                  className="h-8 w-8 text-[10px]"
                />
                <span
                  className={cn(
                    "border-bg absolute right-0 bottom-0 h-2.5 w-2.5 rounded-full border-2",
                    active ? "bg-st-res" : "bg-muted-2",
                  )}
                  title={active ? t("online") : t("offline")}
                  aria-hidden="true"
                />
              </span>
              <span className="flex min-w-0 flex-1 flex-col gap-px">
                <span className="text-text flex min-w-0 items-center gap-2 text-sm leading-4">
                  <span className="min-w-0 flex-1 truncate">{memberDisplayName(member.email)}</span>
                  {self ? (
                    <span className="border-border bg-panel-2 text-muted ml-auto shrink-0 rounded border px-1 py-0.5 text-[9px] leading-none font-medium">
                      {t("currentUserLabel")}
                    </span>
                  ) : null}
                </span>
                <RoleChip role={member.role} showIcon={false} className="text-[11px] leading-3" />
              </span>
            </>
          );
          return (
            <li key={member.user_id}>
              {self ? (
                <div
                  className={itemClassName}
                  aria-label={`${member.email} — ${t("currentUserLabel")} — ${t("online")}`}
                >
                  {content}
                </div>
              ) : (
                <Link
                  href={teamPath(teamId, "messages", member.user_id)}
                  aria-current={current ? "page" : undefined}
                  aria-label={`${tMessages("openConversation", { email: member.email })} — ${
                    active ? t("online") : t("offline")
                  }`}
                  className={itemClassName}
                >
                  {content}
                </Link>
              )}
            </li>
          );
        })}
      </ul>
    </PaneSection>
  );
}

"use client";

import { useTranslations } from "next-intl";
import { Link } from "@/i18n/routing";
import { useIncidents } from "@/lib/queries/incidents";
import type { TeamMember } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { useAuthStore } from "@/store/auth";
import { cn } from "@/lib/utils";

export function WarRoomNavigation({
  activeIncidentId,
  activePeerId,
  inDialog = false,
  members,
  teamId,
}: {
  activeIncidentId?: string;
  activePeerId?: string;
  inDialog?: boolean;
  members: TeamMember[];
  teamId: string;
}) {
  const t = useTranslations("Incidents");
  const currentUserId = useAuthStore((state) => state.user?.id);
  const { data: incidents = [] } = useIncidents(teamId);
  const peers = members.filter((member) => member.user_id !== currentUserId);

  return (
    <aside
      aria-label={t("roomNavigation")}
      className={cn(
        "flex min-h-0 min-w-0 flex-col",
        !inDialog && "bg-panel/25 border-border h-full border-r",
      )}
      data-war-room-navigation="true"
    >
      <nav
        className={cn("min-h-0 flex-1 space-y-6 overflow-y-auto px-2 py-4", !inDialog && "pt-3")}
      >
        <section aria-labelledby="war-room-messages">
          <h2
            id="war-room-messages"
            className="text-muted flex h-7 items-center px-2 text-xs font-semibold tracking-wide uppercase"
          >
            {t("roomDirectMessages")}
          </h2>
          <ul className="mt-1 space-y-0.5">
            {peers.map((member) => {
              const active = member.user_id === activePeerId;
              return (
                <li key={member.user_id}>
                  <Link
                    href={teamPath(teamId, "messages", member.user_id)}
                    aria-current={active ? "page" : undefined}
                    className={cn(
                      "text-muted hover:bg-panel-2 hover:text-text flex min-h-9 w-full min-w-0 items-center gap-2 rounded px-2 py-1.5 text-left text-sm transition-colors",
                      active && "bg-panel-2 text-text",
                    )}
                  >
                    <span className="bg-panel-2 text-muted flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-[10px] font-semibold uppercase">
                      {member.email.slice(0, 2)}
                    </span>
                    <span className="truncate">{member.email}</span>
                  </Link>
                </li>
              );
            })}
          </ul>
        </section>

        <section aria-labelledby="war-room-incidents">
          <Link
            id="war-room-incidents"
            href={teamPath(teamId, "incidents")}
            className="text-muted hover:text-text flex h-7 items-center justify-between px-2 text-xs font-semibold tracking-wide uppercase transition-colors"
          >
            <span>{t("title")}</span>
            <span className="font-mono font-normal">{incidents.length}</span>
          </Link>
          <ul className="mt-1 space-y-0.5">
            {incidents.map((incident) => {
              const active = incident.id === activeIncidentId;
              return (
                <li key={incident.id}>
                  <Link
                    href={teamPath(teamId, "incidents", incident.id)}
                    aria-current={active ? "page" : undefined}
                    className={cn(
                      "text-muted hover:bg-panel-2 hover:text-text flex min-h-9 items-center gap-2 rounded px-2 py-1.5 text-sm transition-colors",
                      active && "bg-panel-2 text-text",
                    )}
                  >
                    <span
                      className={cn(
                        "h-1.5 w-1.5 shrink-0 rounded-full",
                        incident.status === "resolved" ? "bg-muted-2" : "bg-sev-critical",
                      )}
                      aria-hidden="true"
                    />
                    <span className="truncate">{incident.title}</span>
                  </Link>
                </li>
              );
            })}
          </ul>
        </section>
      </nav>
    </aside>
  );
}

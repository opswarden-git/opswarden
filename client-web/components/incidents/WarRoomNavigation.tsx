"use client";

import { useTranslations } from "next-intl";
import { Link } from "@/i18n/routing";
import { useIncidents } from "@/lib/queries/incidents";
import { teamPath } from "@/lib/team-routing";
import { cn } from "@/lib/utils";

export function WarRoomNavigation({
  activeIncidentId,
  inDialog = false,
  teamId,
}: {
  activeIncidentId?: string;
  inDialog?: boolean;
  teamId: string;
}) {
  const t = useTranslations("Incidents");
  const { data: incidents = [] } = useIncidents(teamId);

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
        <section aria-labelledby="war-room-incidents">
          <Link
            id="war-room-incidents"
            href={teamPath(teamId, "incidents")}
            className="text-muted hover:text-text flex h-7 items-center px-2 text-xs font-medium transition-colors"
          >
            <span>
              {t("title")} ({incidents.length})
            </span>
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

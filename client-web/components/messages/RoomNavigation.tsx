"use client";

import { useTranslations } from "next-intl";
import { Link } from "@/i18n/routing";
import { useIncidents, type IncidentListItem } from "@/lib/queries/incidents";
import { useReleases, type ReleaseListItem } from "@/lib/queries/releases";
import { ReleaseStateChip } from "@/components/releases/ReleaseStateChip";
import { IncidentGraphLane } from "@/components/incidents/IncidentGraphLane";
import { teamPath } from "@/lib/team-routing";
import { cn } from "@/lib/utils";

function useSafeIncidents(teamId: string) {
  try {
    return useIncidents(teamId).data ?? [];
  } catch {
    return [];
  }
}

function useSafeReleases(teamId: string) {
  try {
    return useReleases(teamId).data ?? [];
  } catch {
    return [];
  }
}

export function RoomNavigation({
  activeIncidentId,
  inDialog = false,
  teamId,
}: {
  activeIncidentId?: string;
  inDialog?: boolean;
  teamId: string;
}) {
  const t = useTranslations("Incidents");
  const incidents = useSafeIncidents(teamId);
  const releases = useSafeReleases(teamId);

  const byId = new Map(incidents.map((incident) => [incident.id, incident]));
  const claimed = new Set<string>();
  const groups: { release: ReleaseListItem; members: IncidentListItem[] }[] = [];
  for (const release of releases) {
    const members = release.linked_incident_ids
      .map((id) => byId.get(id))
      .filter(
        (incident): incident is IncidentListItem =>
          incident !== undefined && !claimed.has(incident.id),
      );
    if (members.length === 0) continue;
    for (const member of members) claimed.add(member.id);
    groups.push({ release, members });
  }
  const loose = incidents.filter((incident) => !claimed.has(incident.id));

  return (
    <aside
      aria-label={t("roomNavigation")}
      className={cn(
        "flex min-h-0 min-w-0 flex-col",
        !inDialog && "bg-panel/25 border-border h-full border-r",
      )}
      data-war-room-navigation="true"
    >
      <nav className={cn("min-h-0 flex-1 space-y-6 overflow-y-auto py-4", !inDialog && "pt-3")}>
        <section aria-labelledby="war-room-incidents">
          <Link
            id="war-room-incidents"
            href={teamPath(teamId, "incidents")}
            className="text-muted-2 hover:text-text flex h-7 items-center px-4 text-xs font-semibold transition-colors"
          >
            <span className="min-w-0 truncate">
              {t("title")} <span className="tabular-nums opacity-60">({incidents.length})</span>
            </span>
          </Link>
          {groups.map(({ release, members }) => (
            <div key={release.release_id} className="mt-1">
              <Link
                href={teamPath(teamId, "releases", release.release_id)}
                className="text-muted hover:bg-panel-2 hover:text-text flex h-8 items-center gap-2 px-4 text-xs transition-colors"
              >
                <IncidentGraphLane variant="release" runsDown={members.length > 0} />
                <span className="min-w-0 flex-1 truncate">{release.title}</span>
                <ReleaseStateChip state={release.state} />
              </Link>
              <ul>
                {members.map((incident, index) => (
                  <IncidentRow
                    key={incident.id}
                    active={incident.id === activeIncidentId}
                    incident={incident}
                    lane={{ runsDown: index < members.length - 1 }}
                    teamId={teamId}
                    unreadLabel={t("unreadActivity")}
                  />
                ))}
              </ul>
            </div>
          ))}

          <ul className="mt-1">
            {loose.map((incident) => (
              <IncidentRow
                key={incident.id}
                active={incident.id === activeIncidentId}
                incident={incident}
                teamId={teamId}
                unreadLabel={t("unreadActivity")}
              />
            ))}
          </ul>
        </section>
      </nav>
    </aside>
  );
}

export { RoomNavigation as WarRoomNavigation };

function IncidentRow({
  active,
  incident,
  lane,
  teamId,
  unreadLabel,
}: {
  active: boolean;
  incident: IncidentListItem;
  lane?: { runsDown: boolean };
  teamId: string;
  unreadLabel: string;
}) {
  return (
    <li>
      <Link
        href={teamPath(teamId, "incidents", incident.id)}
        aria-current={active ? "page" : undefined}
        className={cn(
          "text-muted hover:bg-panel-2 hover:text-text flex h-8 items-center gap-2 px-4 text-sm transition-colors",
          active && "bg-panel-2 text-text",
          incident.unread && !active && "text-text font-semibold",
        )}
      >
        <IncidentGraphLane
          variant={lane ? "incident" : "loose"}
          runsUp={Boolean(lane)}
          runsDown={lane?.runsDown ?? false}
          tone={lane ? "gold" : "muted"}
        />
        <span className="min-w-0 flex-1 truncate">{incident.title}</span>
        {incident.unread && !active ? (
          <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-white" aria-hidden="true" />
        ) : null}
        {incident.unread ? <span className="sr-only">{unreadLabel}</span> : null}
      </Link>
    </li>
  );
}

"use client";

import { useState } from "react";
import type { ReactNode } from "react";
import { UserRoundCheck } from "lucide-react";
import { useTranslations } from "next-intl";
import { Link } from "@/i18n/routing";
import { type Incident, useAssignIncident } from "@/lib/queries/incidents";
import { useReleases } from "@/lib/queries/releases";
import type { TeamMember } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { ReleaseStateChip } from "@/components/releases/ReleaseStateChip";
import { MemberAvatar } from "@/components/teams/MemberAvatar";
import { IconButton } from "@/components/ui/Button";
import { SeverityChip } from "./SeverityChip";
import { StateChip } from "./StateChip";

export function IncidentContextPanel({
  canAssign,
  incident,
  members,
  watcherIds,
  commands,
  inDialog = false,
}: {
  canAssign: boolean;
  incident: Incident;
  members: TeamMember[];
  watcherIds: string[];
  commands?: ReactNode;
  inDialog?: boolean;
}) {
  const t = useTranslations("Incidents");
  const tErr = useTranslations("errors");
  const assignIncident = useAssignIncident();
  const {
    data: releases,
    error: releasesError,
    isLoading: releasesLoading,
  } = useReleases(incident.team_id);
  const [assigneeId, setAssigneeId] = useState("");
  const eligibleAssignees = members.filter(
    (member) => member.role === "manager" || member.role === "responder",
  );
  const memberById = new Map(members.map((member) => [member.user_id, member]));
  const assignee = incident.assignee ? memberById.get(incident.assignee) : undefined;
  const selectedAssignee = assigneeId || incident.assignee || "";
  const linkedReleases = (releases ?? []).filter((release) =>
    release.linked_incident_ids.includes(incident.id),
  );
  const watcherMembers = [...new Set(watcherIds)].map((userId) => ({
    userId,
    email: memberById.get(userId)?.email ?? t("teamMember"),
  }));
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));

  const assign = () => {
    if (!selectedAssignee || selectedAssignee === incident.assignee) return;
    assignIncident.mutate({ incidentId: incident.id, assigneeId: selectedAssignee });
  };

  return (
    <aside
      className={
        inDialog
          ? "min-w-0"
          : "bg-panel/25 border-border flex h-full min-w-0 flex-col overflow-y-auto border-l"
      }
      aria-label={t("incidentContext")}
      data-war-room-context="true"
    >
      <div
        className={
          inDialog
            ? "mb-5 flex items-center justify-between gap-3"
            : "border-border flex items-center justify-between gap-3 border-b p-3"
        }
      >
        <div className="flex flex-wrap gap-2">
          <StateChip status={incident.status} />
          <SeverityChip severity={incident.severity} />
        </div>

        {watcherMembers.length > 0 ? (
          <ul
            aria-label={t("watchersTitle")}
            className="flex shrink-0 -space-x-2"
            title={t("watchersTitle")}
          >
            {watcherMembers.slice(0, 4).map((watcher) => (
              <li key={watcher.userId} title={watcher.email} className="relative">
                <MemberAvatar
                  email={watcher.email}
                  className="border-bg h-7 w-7 border-2 text-[9px]"
                />
                <span className="bg-st-res border-bg absolute right-0 bottom-0 h-2 w-2 rounded-full border" />
                <span className="sr-only">{watcher.email}</span>
              </li>
            ))}
            {watcherMembers.length > 4 ? (
              <li className="surface-subtle border-bg text-muted flex h-7 w-7 items-center justify-center rounded-full border-2 text-[9px] font-medium">
                +{watcherMembers.length - 4}
              </li>
            ) : null}
          </ul>
        ) : null}
      </div>

      {commands ? (
        <div className={inDialog ? "mb-5" : "border-border border-b p-4"}>{commands}</div>
      ) : null}

      <div className={inDialog ? "space-y-5 text-sm" : "space-y-5 p-4 text-sm"}>
        {canAssign ? (
          <div>
            <div className="flex items-center gap-2">
              <label className="min-w-0 flex-1">
                <span className="sr-only">{t("changeAssignee")}</span>
                <select
                  value={selectedAssignee}
                  onChange={(event) => setAssigneeId(event.target.value)}
                  className="ow-input h-9 w-full min-w-0 rounded-md px-3 text-sm"
                >
                  <option value="">{t("unassigned")}</option>
                  {eligibleAssignees.map((member) => (
                    <option key={member.user_id} value={member.user_id}>
                      {member.email}
                    </option>
                  ))}
                </select>
              </label>
              <IconButton
                label={t("assign")}
                size="sm"
                onClick={assign}
                loading={assignIncident.isPending}
                disabled={!selectedAssignee || selectedAssignee === incident.assignee}
              >
                <UserRoundCheck className="h-4 w-4" aria-hidden="true" />
              </IconButton>
            </div>
            {assignIncident.error ? (
              <p className="text-sev-critical mt-2 text-xs" role="alert">
                {errorText(assignIncident.error.message)}
              </p>
            ) : null}
          </div>
        ) : (
          <div className="flex min-w-0 items-center gap-2" title={t("colAssignee")}>
            <MemberAvatar
              email={assignee?.email ?? t("unassigned")}
              className="h-7 w-7 text-[9px]"
            />
            <span className="text-text truncate">{assignee?.email ?? t("unassigned")}</span>
          </div>
        )}

        {releasesLoading ? (
          <div className="bg-panel-2 h-8 animate-pulse rounded-md" />
        ) : releasesError ? (
          <p className="text-sev-critical text-xs" role="alert">
            {t("failedToLoadLinkedReleases")}
          </p>
        ) : linkedReleases.length > 0 ? (
          <section aria-labelledby="linked-release-actions">
            <h2
              id="linked-release-actions"
              className="text-muted mb-1 text-[10px] font-semibold tracking-wide uppercase"
            >
              {t("linkedReleases")}
            </h2>
            <ul className="space-y-1">
              {linkedReleases.map((release) => (
                <li key={release.release_id}>
                  <Link
                    href={teamPath(incident.team_id, "releases", release.release_id)}
                    className="text-muted hover:bg-panel-2 hover:text-text flex min-w-0 items-center gap-2 rounded-md px-2 py-2 transition-colors"
                  >
                    <span className="text-text min-w-0 flex-1 truncate">{release.title}</span>
                    <ReleaseStateChip state={release.state} />
                  </Link>
                </li>
              ))}
            </ul>
          </section>
        ) : null}
      </div>
    </aside>
  );
}

"use client";

import { useState } from "react";
import { UserPlus } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { Link } from "@/i18n/routing";
import { type Incident, useAssignIncident } from "@/lib/queries/incidents";
import { useReleases } from "@/lib/queries/releases";
import type { Team, TeamMember } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { ReleaseStateChip } from "@/components/releases/ReleaseStateChip";
import { Button } from "@/components/ui/Button";
import { SeverityChip } from "./SeverityChip";

export function IncidentContextPanel({
  canAssign,
  incident,
  members,
  team,
  watcherIds,
  inDialog = false,
}: {
  canAssign: boolean;
  incident: Incident;
  members: TeamMember[];
  team: Team | undefined;
  watcherIds: string[];
  inDialog?: boolean;
}) {
  const t = useTranslations("Incidents");
  const tErr = useTranslations("errors");
  const locale = useLocale();
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
  const formatDate = (value: string) =>
    new Intl.DateTimeFormat(locale, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(value));
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));

  const assign = () => {
    if (!selectedAssignee || selectedAssignee === incident.assignee) return;
    assignIncident.mutate({ incidentId: incident.id, assigneeId: selectedAssignee });
  };

  return (
    <aside
      className={
        inDialog ? "min-w-0" : "surface border-border min-w-0 rounded-md border p-4 sm:p-5"
      }
      aria-labelledby="context-title"
    >
      {!inDialog && (
        <h2 id="context-title" className="text-text text-base font-semibold">
          {t("incidentContext")}
        </h2>
      )}

      <dl className={inDialog ? "space-y-5 text-sm" : "mt-5 space-y-5 text-sm"}>
        <div>
          <dt className="text-muted text-xs font-medium">{t("teamLabel")}</dt>
          <dd className="mt-1 min-w-0">
            <Link
              href={teamPath(incident.team_id, "overview")}
              className="text-text hover:text-gold block truncate transition-colors"
            >
              {team?.name ?? t("teamMember")}
            </Link>
          </dd>
        </div>

        <div>
          <dt className="text-muted text-xs font-medium">{t("colAssignee")}</dt>
          <dd className="text-text mt-1 break-words">{assignee?.email ?? t("unassigned")}</dd>
        </div>

        {canAssign ? (
          <div>
            <dt className="text-muted text-xs font-medium">{t("changeAssignee")}</dt>
            <dd className="mt-2 space-y-2">
              <label className="block">
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
              <Button
                size="sm"
                fullWidth
                onClick={assign}
                loading={assignIncident.isPending}
                disabled={!selectedAssignee || selectedAssignee === incident.assignee}
              >
                <UserPlus className="h-4 w-4" aria-hidden="true" />
                {t("assign")}
              </Button>
              {assignIncident.error ? (
                <p className="text-sev-critical text-xs" role="alert">
                  {errorText(assignIncident.error.message)}
                </p>
              ) : null}
            </dd>
          </div>
        ) : null}

        <div>
          <dt className="text-muted text-xs font-medium">{t("severity")}</dt>
          <dd className="mt-1">
            <SeverityChip severity={incident.severity} />
          </dd>
        </div>

        <div>
          <dt className="text-muted text-xs font-medium">{t("createdAt")}</dt>
          <dd className="text-text mt-1">
            <time dateTime={incident.created_at}>{formatDate(incident.created_at)}</time>
          </dd>
        </div>

        <div>
          <dt className="text-muted text-xs font-medium">{t("updatedAt")}</dt>
          <dd className="text-text mt-1">
            <time dateTime={incident.updated_at}>{formatDate(incident.updated_at)}</time>
          </dd>
        </div>

        <div>
          <dt className="text-muted text-xs font-medium">{t("fieldDescription")}</dt>
          <dd className="text-text mt-1 leading-5 break-words whitespace-pre-wrap">
            {incident.description || t("noDescription")}
          </dd>
        </div>

        <div>
          <dt className="text-muted text-xs font-medium">{t("linkedReleases")}</dt>
          <dd className="mt-2">
            {releasesLoading ? (
              <span className="text-muted">{t("loadingLinkedReleases")}</span>
            ) : releasesError ? (
              <span className="text-sev-critical">{t("failedToLoadLinkedReleases")}</span>
            ) : linkedReleases.length === 0 ? (
              <span className="text-muted">{t("noLinkedReleases")}</span>
            ) : (
              <ul className="space-y-2">
                {linkedReleases.map((release) => (
                  <li key={release.release_id}>
                    <Link
                      href={teamPath(incident.team_id, "releases", release.release_id)}
                      className="surface-subtle border-border hover:border-gold/40 flex min-w-0 items-center gap-2 rounded-md border p-2 transition-colors"
                    >
                      <span className="text-text min-w-0 flex-1 truncate">{release.title}</span>
                      <ReleaseStateChip state={release.state} />
                    </Link>
                  </li>
                ))}
              </ul>
            )}
          </dd>
        </div>

        <div>
          <dt className="text-muted text-xs font-medium">{t("watchersTitle")}</dt>
          <dd className="text-text mt-2">
            {watcherMembers.length === 0 ? (
              <span className="text-muted">{t("noWatchers")}</span>
            ) : (
              <ul aria-label={t("watchersTitle")} className="space-y-1">
                {watcherMembers.map((watcher) => (
                  <li key={watcher.userId} className="flex min-w-0 items-center gap-2">
                    <span className="bg-st-res h-2 w-2 shrink-0 rounded-full" aria-hidden="true" />
                    <span className="min-w-0 truncate">{watcher.email}</span>
                  </li>
                ))}
              </ul>
            )}
          </dd>
        </div>
      </dl>
    </aside>
  );
}

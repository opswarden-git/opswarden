"use client";

import { useState } from "react";
import type { ReactNode } from "react";
import { Check, CircleHelp } from "lucide-react";
import { useTranslations } from "next-intl";
import { Link } from "@/i18n/routing";
import { type Incident, useAssignIncident } from "@/lib/queries/incidents";
import { useReleases } from "@/lib/queries/releases";
import type { TeamMember } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { cn } from "@/lib/utils";
import { ReleaseStateChip } from "@/components/releases/ReleaseStateChip";
import { TeamPresenceList } from "@/components/messages/TeamPresenceList";
import { MemberAvatar } from "@/components/teams/MemberAvatar";
import { IconButton } from "@/components/ui/Button";
import { PaneSection } from "@/components/ui/PaneSection";
import { Skeleton } from "@/components/ui/Skeleton";
import { SeverityChip } from "./SeverityChip";
import { StateChip } from "./StateChip";

export function IncidentContextPanel({
  canAssign,
  incident,
  members,
  watcherIds,
  commands,
  dangerCommands,
  inDialog = false,
}: {
  canAssign: boolean;
  incident: Incident;
  members: TeamMember[];
  watcherIds: string[];
  commands?: ReactNode;
  dangerCommands?: ReactNode;
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
  const eligibleAssignees = members.filter((member) => member.can_be_assigned_incident);
  const memberById = new Map(members.map((member) => [member.user_id, member]));
  const assignee = incident.assignee ? memberById.get(incident.assignee) : undefined;
  const selectedAssignee = assigneeId || incident.assignee || "";
  const linkedReleases = (releases ?? []).filter((release) =>
    release.linked_incident_ids.includes(incident.id),
  );
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));
  const actionRequired = incident.status === "open";
  const assigneeRequired = !incident.assignee;

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
      <div className={cn("divide-border-muted divide-y text-sm", inDialog ? "" : "p-2")}>
        <PaneSection title={t("details")} defaultOpen={false}>
          <div className="space-y-3 px-2">
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted text-xs font-medium">{t("colStatus")}</span>
              <StateChip status={incident.status} />
            </div>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted text-xs font-medium">{t("severity")}</span>
              <SeverityChip severity={incident.severity} />
            </div>
          </div>
        </PaneSection>

        {commands || dangerCommands ? (
          <PaneSection
            defaultOpen={true}
            title={
              <SectionTitle
                label={t("moreActions")}
                attention={actionRequired}
                attentionLabel={t("actionRequired")}
              />
            }
          >
            <div className="space-y-3 px-2">
              {commands}
              {dangerCommands}
            </div>
          </PaneSection>
        ) : null}

        <PaneSection
          defaultOpen={true}
          title={
            <SectionTitle
              label={t("colAssignee")}
              attention={assigneeRequired}
              attentionLabel={t("assigneeRequired")}
            />
          }
        >
          <div className="space-y-2 px-2">
            {canAssign ? (
              <div>
                <div className="flex min-w-0 items-center gap-2">
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
                    variant="ghost"
                    className="text-muted hover:text-st-res focus-visible:text-st-res hover:bg-transparent focus-visible:bg-transparent"
                    onClick={assign}
                    loading={assignIncident.isPending}
                    disabled={!selectedAssignee || selectedAssignee === incident.assignee}
                  >
                    <Check className="h-5 w-5" strokeWidth={2} aria-hidden="true" />
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
                  role={assignee?.role}
                  className="h-7 w-7 text-[9px]"
                />
                <span className="text-text truncate">{assignee?.email ?? t("unassigned")}</span>
              </div>
            )}
          </div>
        </PaneSection>

        {releasesLoading || releasesError || linkedReleases.length > 0 ? (
          <PaneSection title={t("linkedReleases")} defaultOpen={false}>
            <div className="px-2">
              {releasesLoading ? (
                <div className="space-y-2" aria-busy="true" aria-label={t("linkedReleases")}>
                  <Skeleton className="h-3 w-24" />
                  <div className="flex items-center gap-2 py-2">
                    <Skeleton className="h-4 min-w-0 flex-1" />
                    <Skeleton className="h-5 w-20 rounded-full" />
                  </div>
                </div>
              ) : releasesError ? (
                <p className="text-sev-critical text-xs" role="alert">
                  {t("failedToLoadLinkedReleases")}
                </p>
              ) : (
                <ul className="space-y-1">
                  {linkedReleases.map((release) => (
                    <li key={release.release_id}>
                      <Link
                        href={teamPath(incident.team_id, "releases", release.release_id)}
                        className="text-muted hover:text-gold flex min-w-0 items-center gap-2 py-2 transition-colors"
                      >
                        <span className="text-text min-w-0 flex-1 truncate">{release.title}</span>
                        <ReleaseStateChip state={release.state} />
                      </Link>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </PaneSection>
        ) : null}

        <TeamPresenceList
          className={inDialog ? "px-0" : undefined}
          members={members}
          presentUserIds={watcherIds}
          teamId={incident.team_id}
        />
      </div>
    </aside>
  );
}

function SectionTitle({
  attention,
  attentionLabel,
  label,
}: {
  attention: boolean;
  attentionLabel: string;
  label: string;
}) {
  return (
    <span className="flex min-w-0 items-center gap-1.5">
      <span className="truncate">{label}</span>
      {attention ? (
        <span className="text-gold shrink-0" title={attentionLabel} aria-label={attentionLabel}>
          <CircleHelp className="h-3.5 w-3.5" strokeWidth={2} aria-hidden="true" />
        </span>
      ) : null}
    </span>
  );
}

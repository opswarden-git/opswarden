"use client";

import React, { useState } from "react";
import { CheckCircle2, Clock, Info, ShieldAlert, Trash2 } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { Link, useRouter } from "@/i18n/routing";
import { type IncidentTransition, deriveIncidentActions } from "@/lib/capabilities";
import { useDeleteIncident, useIncident, useUpdateIncidentStatus } from "@/lib/queries/incidents";
import { useTeamMembers, useTeams } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { useWatchers, useWsStore } from "@/lib/ws";
import { IncidentActivity } from "@/components/incidents/IncidentActivity";
import { IncidentContextPanel } from "@/components/incidents/IncidentContextPanel";
import { deriveIncidentHeaderActions } from "@/components/incidents/incident-detail";
import { SeverityChip } from "@/components/incidents/SeverityChip";
import { StateChip } from "@/components/incidents/StateChip";
import { PageContent } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { ActionMenu } from "@/components/ui/ActionMenu";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { Dialog } from "@/components/ui/Dialog";

import { formatRelativeAge } from "@/lib/utils";

function IncidentBreadcrumb({
  currentHref,
  incidentsHref,
  shortId,
  teamHref,
  teamName,
}: {
  currentHref: string;
  incidentsHref: string;
  shortId: string;
  teamHref: string;
  teamName: string;
}) {
  const t = useTranslations("Incidents");

  return (
    <nav aria-label={t("breadcrumbLabel")} className="min-w-0 text-sm">
      <ol className="text-muted flex min-w-0 items-center gap-2">
        <li className="min-w-0 truncate">
          <Link href={teamHref} className="hover:text-text transition-colors">
            {teamName}
          </Link>
        </li>
        <li aria-hidden="true">/</li>
        <li>
          <Link href={incidentsHref} className="hover:text-text transition-colors">
            {t("title")}
          </Link>
        </li>
        <li aria-hidden="true">/</li>
        <li className="min-w-0 truncate">
          <Link
            href={currentHref}
            aria-current="page"
            className="text-text font-medium transition-colors"
          >
            {t("incidentBreadcrumb", { id: shortId })}
          </Link>
        </li>
      </ol>
    </nav>
  );
}

export function IncidentDetailPage({ incidentId, teamId }: { incidentId: string; teamId: string }) {
  const t = useTranslations("Incidents");
  const tErr = useTranslations("errors");
  const locale = useLocale();
  const router = useRouter();
  const { data: incident, isLoading, error } = useIncident(incidentId);
  const { data: teams } = useTeams();
  const { data: members } = useTeamMembers(incident?.team_id);
  const updateStatus = useUpdateIncidentStatus();
  const deleteIncident = useDeleteIncident();
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [isContextOpen, setIsContextOpen] = useState(false);
  const watch = useWsStore((state) => state.watch);
  const unwatch = useWsStore((state) => state.unwatch);
  const watchers = useWatchers(incidentId);

  React.useEffect(() => {
    watch(incidentId);
    return () => unwatch(incidentId);
  }, [incidentId, watch, unwatch]);

  React.useEffect(() => {
    if (!incident || teamId === incident.team_id) return;
    router.replace(teamPath(incident.team_id, "incidents", incident.id));
  }, [incident, router, teamId]);

  const incidentsHref = teamPath(incident?.team_id ?? teamId, "incidents");

  if (isLoading) {
    return (
      <PageLayout width="workspace">
        <PageHeader title={t("title")} />
        <PageContent
          state="loading"
          loadingFallback={
            <div className="grid grid-cols-1 gap-6 lg:grid-cols-[minmax(0,1fr)_20rem]">
              <div className="surface h-96 animate-pulse rounded-md" />
              <div className="surface h-72 animate-pulse rounded-md" />
            </div>
          }
        />
      </PageLayout>
    );
  }

  if (error || !incident) {
    return (
      <PageLayout width="workspace">
        <PageHeader title={t("title")} />
        <PageContent
          state="error"
          errorFallback={<Alert tone="danger">{t("failedToLoadIncident")}</Alert>}
        />
      </PageLayout>
    );
  }

  const currentTeam = teams?.find((team) => team.team_id === incident.team_id);
  const actions = deriveIncidentActions(currentTeam?.role ?? "observer", incident.status);
  const headerActions = deriveIncidentHeaderActions(actions.transitions);
  const memberById = new Map((members ?? []).map((member) => [member.user_id, member]));
  const assignee = incident.assignee ? memberById.get(incident.assignee) : undefined;
  const people = Object.fromEntries(
    (members ?? []).map((member) => [member.user_id, member.email]),
  );
  const teamName = currentTeam?.name ?? t("teamMember");
  const currentHref = teamPath(incident.team_id, "incidents", incident.id);
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));

  const transitionLabel = (transition: IncidentTransition) =>
    transition === "acknowledged"
      ? t("acknowledge")
      : transition === "escalated"
        ? t("escalate")
        : t("resolve");

  const transitionIcon = (transition: IncidentTransition) =>
    transition === "acknowledged" ? (
      <Clock className="h-4 w-4" aria-hidden="true" />
    ) : transition === "escalated" ? (
      <ShieldAlert className="h-4 w-4" aria-hidden="true" />
    ) : (
      <CheckCircle2 className="h-4 w-4" aria-hidden="true" />
    );

  const transitionButton = (transition: IncidentTransition, primary: boolean) => (
    <Button
      key={transition}
      variant={primary ? "primary" : "secondary"}
      size="lg"
      loading={updateStatus.isPending}
      onClick={() => updateStatus.mutate({ incidentId: incident.id, status: transition })}
    >
      {transitionIcon(transition)}
      {transitionLabel(transition)}
    </Button>
  );

  const deleteCurrentIncident = () =>
    deleteIncident.mutate(incident.id, {
      onSuccess: () => router.push(teamPath(incident.team_id, "incidents")),
    });

  return (
    <PageLayout width="workspace">
      <IncidentBreadcrumb
        currentHref={currentHref}
        incidentsHref={incidentsHref}
        shortId={incident.id.slice(0, 8)}
        teamHref={teamPath(incident.team_id, "overview")}
        teamName={teamName}
      />

      <PageHeader
        title={incident.title}
        metadata={
          <div className="flex flex-wrap items-center gap-2">
            <StateChip status={incident.status} />
            <SeverityChip severity={incident.severity} />
            <span className="text-muted">·</span>
            <span>{assignee?.email ?? t("unassigned")}</span>
            <span className="text-muted">·</span>
            <time dateTime={incident.created_at}>
              {formatRelativeAge(incident.created_at, locale)}
            </time>
          </div>
        }
        actions={
          <>
            {headerActions.secondary ? transitionButton(headerActions.secondary, false) : null}
            {headerActions.primary ? transitionButton(headerActions.primary, true) : null}
            <div className="lg:hidden">
              <Button variant="secondary" size="lg" onClick={() => setIsContextOpen(true)}>
                <Info className="h-4 w-4" aria-hidden="true" />
                {t("incidentContext")}
              </Button>
            </div>
            {actions.canDelete ? (
              <ActionMenu
                label={t("moreActions")}
                items={[
                  {
                    id: "delete",
                    label: t("deleteIncident"),
                    icon: Trash2,
                    tone: "danger",
                    onSelect: () => {
                      deleteIncident.reset();
                      setDeleteOpen(true);
                    },
                  },
                ]}
              />
            ) : null}
          </>
        }
      />

      {updateStatus.error ? (
        <Alert tone="danger">{errorText(updateStatus.error.message)}</Alert>
      ) : null}

      <PageContent>
        <div className="grid grid-cols-1 items-start gap-6 lg:grid-cols-[minmax(0,1fr)_20rem]">
          <IncidentActivity
            incidentId={incident.id}
            canCompose={actions.canWriteTimeline}
            people={people}
          />

          <div className="hidden lg:block">
            <IncidentContextPanel
              incident={incident}
              team={currentTeam}
              members={members ?? []}
              watcherIds={watchers}
              canAssign={actions.canAssign}
            />
          </div>
        </div>
      </PageContent>

      <ConfirmDialog
        open={deleteOpen}
        title={t("deleteIncident")}
        description={t("deleteIncidentConfirm", { title: incident.title })}
        confirmLabel={t("deleteIncident")}
        cancelLabel={t("cancel")}
        pendingLabel={t("processing")}
        danger
        requireType="DELETE"
        requireTypeLabel={t("deleteConfirmationInput")}
        pending={deleteIncident.isPending}
        error={deleteIncident.error ? errorText(deleteIncident.error.message) : null}
        onConfirm={deleteCurrentIncident}
        onClose={() => setDeleteOpen(false)}
      />

      <Dialog
        open={isContextOpen}
        onOpenChange={setIsContextOpen}
        variant="sheet"
        title={t("incidentContext")}
        description={incident.title}
      >
        <IncidentContextPanel
          inDialog
          incident={incident}
          team={currentTeam}
          members={members ?? []}
          watcherIds={watchers}
          canAssign={actions.canAssign}
        />
      </Dialog>
    </PageLayout>
  );
}

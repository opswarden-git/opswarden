"use client";

import React, { useState } from "react";
import { PanelLeftOpen, PanelRightOpen, Trash2 } from "lucide-react";
import { useTranslations } from "next-intl";
import { useRouter } from "@/i18n/routing";
import { type IncidentTransition, deriveIncidentActions } from "@/lib/capabilities";
import { useDeleteIncident, useIncident, useUpdateIncidentStatus } from "@/lib/queries/incidents";
import { useTeamMembers, useTeams } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { useWatchers, useWsStore } from "@/lib/ws";
import { IncidentActivity } from "@/components/incidents/IncidentActivity";
import { IncidentContextPanel } from "@/components/incidents/IncidentContextPanel";
import { WarRoomNavigation } from "@/components/incidents/WarRoomNavigation";
import { deriveIncidentHeaderActions } from "@/components/incidents/incident-detail";
import { PageContent } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { RailToggle } from "@/components/layout/RailToggle";
import { ActionMenu } from "@/components/ui/ActionMenu";
import { Alert } from "@/components/ui/Alert";
import { Button, IconButton } from "@/components/ui/Button";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { Dialog } from "@/components/ui/Dialog";
import { cn } from "@/lib/utils";

export function IncidentDetailPage({ incidentId, teamId }: { incidentId: string; teamId: string }) {
  const t = useTranslations("Incidents");
  const tErr = useTranslations("errors");
  const router = useRouter();
  const { data: incident, isLoading, error } = useIncident(incidentId);
  const { data: teams } = useTeams();
  const { data: members } = useTeamMembers(incident?.team_id);
  const updateStatus = useUpdateIncidentStatus();
  const deleteIncident = useDeleteIncident();
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [isContextOpen, setIsContextOpen] = useState(false);
  const [isRoomsOpen, setIsRoomsOpen] = useState(false);
  const [isRoomsRailOpen, setIsRoomsRailOpen] = useState(true);
  const [isContextRailOpen, setIsContextRailOpen] = useState(true);
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

  if (isLoading) {
    return (
      <PageLayout>
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
      <PageLayout>
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
  const people = Object.fromEntries(
    (members ?? []).map((member) => [member.user_id, member.email]),
  );
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));

  const transitionLabel = (transition: IncidentTransition) =>
    transition === "acknowledged"
      ? t("acknowledge")
      : transition === "escalated"
        ? t("escalate")
        : t("resolve");

  const transitionButton = (transition: IncidentTransition, primary: boolean) => (
    <Button
      key={transition}
      variant={primary ? "primary" : "secondary"}
      size="sm"
      loading={updateStatus.isPending}
      onClick={() => updateStatus.mutate({ incidentId: incident.id, status: transition })}
    >
      {transitionLabel(transition)}
    </Button>
  );

  const deleteCurrentIncident = () =>
    deleteIncident.mutate(incident.id, {
      onSuccess: () => router.push(teamPath(incident.team_id, "incidents")),
    });

  const commands = (
    <div className="space-y-3">
      <div className="flex flex-wrap gap-2">
        {headerActions.secondary ? transitionButton(headerActions.secondary, false) : null}
        {headerActions.primary ? transitionButton(headerActions.primary, true) : null}
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
      </div>
      {updateStatus.error ? (
        <p className="text-sev-critical text-xs" role="alert">
          {errorText(updateStatus.error.message)}
        </p>
      ) : null}
    </div>
  );

  return (
    <PageLayout fill className="max-w-none gap-0 px-0 pt-0 pb-0 sm:px-0 md:px-0 md:pt-0 md:pb-0">
      <PageContent className="flex min-h-0 flex-1 flex-col">
        <div
          className={cn(
            "border-border grid min-h-0 flex-1 grid-cols-1 overflow-hidden border-y",
            isContextRailOpen
              ? "lg:grid-cols-[minmax(0,1fr)_19rem]"
              : "lg:grid-cols-[minmax(0,1fr)_1rem]",
            isRoomsRailOpen && !isContextRailOpen && "xl:grid-cols-[14rem_minmax(0,1fr)_1rem]",
            !isRoomsRailOpen && isContextRailOpen && "xl:grid-cols-[1rem_minmax(0,1fr)_19rem]",
            !isRoomsRailOpen && !isContextRailOpen && "xl:grid-cols-[1rem_minmax(0,1fr)_1rem]",
            isRoomsRailOpen && isContextRailOpen && "xl:grid-cols-[14rem_minmax(0,1fr)_19rem]",
          )}
        >
          <div
            className={cn(
              "relative hidden min-h-0 xl:block",
              !isRoomsRailOpen && "border-border border-r",
            )}
            data-rooms-rail-open={isRoomsRailOpen ? "true" : "false"}
          >
            {isRoomsRailOpen ? (
              <WarRoomNavigation
                activeIncidentId={incident.id}
                members={members ?? []}
                teamId={incident.team_id}
              />
            ) : null}
            <RailToggle
              className="top-1/2 right-0 -translate-y-1/2"
              direction={isRoomsRailOpen ? "left" : "right"}
              label={t(isRoomsRailOpen ? "collapseRooms" : "expandRooms")}
              onClick={() => setIsRoomsRailOpen((open) => !open)}
            />
          </div>

          <main className="relative flex min-h-0 min-w-0 flex-col">
            <h1 className="sr-only">{incident.title}</h1>
            <div className="absolute top-2 right-2 z-20 flex items-center gap-1">
              <IconButton
                className="xl:hidden"
                label={t("rooms")}
                size="sm"
                variant="ghost"
                onClick={() => setIsRoomsOpen(true)}
              >
                <PanelLeftOpen className="h-4 w-4" aria-hidden="true" />
              </IconButton>
              <IconButton
                className="lg:hidden"
                label={t("details")}
                size="sm"
                variant="ghost"
                onClick={() => setIsContextOpen(true)}
              >
                <PanelRightOpen className="h-4 w-4" aria-hidden="true" />
              </IconButton>
            </div>

            <IncidentActivity
              incidentId={incident.id}
              canCompose={actions.canWriteTimeline}
              people={people}
            />
          </main>

          <div
            className={cn(
              "relative hidden min-h-0 lg:block",
              !isContextRailOpen && "border-border border-l",
            )}
            data-context-rail-open={isContextRailOpen ? "true" : "false"}
          >
            <RailToggle
              className="top-1/2 left-0 -translate-y-1/2"
              direction={isContextRailOpen ? "right" : "left"}
              label={t(isContextRailOpen ? "collapseContext" : "expandContext")}
              onClick={() => setIsContextRailOpen((open) => !open)}
            />
            {isContextRailOpen ? (
              <IncidentContextPanel
                incident={incident}
                members={members ?? []}
                watcherIds={watchers}
                canAssign={actions.canAssign}
                commands={commands}
              />
            ) : null}
          </div>
        </div>
      </PageContent>

      <ConfirmDialog
        open={deleteOpen}
        title={t("deleteIncident")}
        description={t("deleteIncidentConfirm", { title: incident.title })}
        confirmLabel={t("deleteIncident")}
        cancelLabel={t("cancel")}
        intent="destructive"
        pendingLabel={t("processing")}
        requireType="DELETE"
        requireTypeLabel={t("deleteConfirmationInput")}
        pending={deleteIncident.isPending}
        error={deleteIncident.error ? errorText(deleteIncident.error.message) : null}
        onConfirm={deleteCurrentIncident}
        onClose={() => setDeleteOpen(false)}
      />

      <Dialog
        open={isRoomsOpen}
        onOpenChange={setIsRoomsOpen}
        variant="sheet"
        title={t("warRoom")}
        description={incident.title}
      >
        <WarRoomNavigation
          inDialog
          activeIncidentId={incident.id}
          members={members ?? []}
          teamId={incident.team_id}
        />
      </Dialog>

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
          members={members ?? []}
          watcherIds={watchers}
          canAssign={actions.canAssign}
          commands={commands}
        />
      </Dialog>
    </PageLayout>
  );
}

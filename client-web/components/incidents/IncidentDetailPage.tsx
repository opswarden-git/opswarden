"use client";

import React, { useState } from "react";
import { PanelLeftOpen, PanelRightOpen } from "lucide-react";
import { useTranslations } from "next-intl";
import { useRouter } from "@/i18n/routing";
import {
  type IncidentTransition,
  useDeleteIncident,
  useIncident,
  useUpdateIncidentStatus,
} from "@/lib/queries/incidents";
import { useTeamMembers } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { useWatchers } from "@/lib/ws";
import { IncidentActivity } from "@/components/incidents/IncidentActivity";
import { IncidentContextPanel } from "@/components/incidents/IncidentContextPanel";
import { WarRoomNavigation } from "@/components/incidents/WarRoomNavigation";
import { ConversationRoomSkeleton } from "@/components/messages/ConversationRoomSkeleton";
import { deriveIncidentHeaderActions } from "@/components/incidents/incident-detail";
import { PageContent } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { RailToggle } from "@/components/layout/RailToggle";
import { Alert } from "@/components/ui/Alert";
import { actionButtonClassNames, Button, IconButton } from "@/components/ui/Button";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { Dialog } from "@/components/ui/Dialog";
import { cn } from "@/lib/utils";
import { useConversationRoom } from "@/lib/useConversationRoom";

export function IncidentDetailPage({ incidentId, teamId }: { incidentId: string; teamId: string }) {
  const t = useTranslations("Incidents");
  const tErr = useTranslations("errors");
  const router = useRouter();
  const { data: incident, isLoading, error } = useIncident(incidentId);
  const { data: members } = useTeamMembers(incident?.team_id);
  const updateStatus = useUpdateIncidentStatus();
  const deleteIncident = useDeleteIncident();
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [isContextOpen, setIsContextOpen] = useState(false);
  const [isRoomsOpen, setIsRoomsOpen] = useState(false);
  const [isRoomsRailOpen, setIsRoomsRailOpen] = useState(true);
  const [isContextRailOpen, setIsContextRailOpen] = useState(true);
  const watchers = useWatchers(incidentId);
  useConversationRoom({ kind: "incident", id: incidentId });

  React.useEffect(() => {
    if (!incident || teamId === incident.team_id) return;
    router.replace(teamPath(incident.team_id, "incidents", incident.id));
  }, [incident, router, teamId]);

  if (isLoading) {
    return (
      <PageLayout fill className="max-w-none gap-0 px-0 pt-0 pb-0 sm:px-0 md:px-0 md:pt-0 md:pb-0">
        <PageContent className="flex min-h-0 flex-1 flex-col">
          <ConversationRoomSkeleton context="incident" label={t("loadingActivity")} />
        </PageContent>
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

  const actions = incident.actions;
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
      className={actionButtonClassNames()}
      fullWidth
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

  const hasTransitions = Boolean(headerActions.secondary || headerActions.primary);
  const commands = hasTransitions ? (
    <div className="space-y-2">
      {headerActions.secondary ? transitionButton(headerActions.secondary, false) : null}
      {headerActions.primary ? transitionButton(headerActions.primary, true) : null}
      {updateStatus.error ? (
        <p className="text-sev-critical text-xs" role="alert">
          {errorText(updateStatus.error.message)}
        </p>
      ) : null}
    </div>
  ) : null;
  const dangerCommands = actions.canDelete ? (
    <Button
      variant="danger"
      className={actionButtonClassNames()}
      fullWidth
      onClick={() => {
        deleteIncident.reset();
        setDeleteOpen(true);
      }}
    >
      {t("deleteIncident")}
    </Button>
  ) : null;

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
              <WarRoomNavigation activeIncidentId={incident.id} teamId={incident.team_id} />
            ) : null}
            <RailToggle
              side="right"
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
                label={t("incidentContext")}
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
              side="left"
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
                dangerCommands={dangerCommands}
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
        <WarRoomNavigation inDialog activeIncidentId={incident.id} teamId={incident.team_id} />
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
          dangerCommands={dangerCommands}
        />
      </Dialog>
    </PageLayout>
  );
}

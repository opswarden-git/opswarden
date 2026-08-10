"use client";

import { useLocale, useTranslations } from "next-intl";
import { useState } from "react";
import { useRouter } from "@/i18n/routing";
import { deriveCapabilities } from "@/lib/capabilities";
import { type Team, useDeleteTeam, useLeaveTeam, useTeams } from "@/lib/queries/teams";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageLayout } from "@/components/layout/PageLayout";
import { IdentityHeader } from "@/components/settings/SettingsPrimitives";
import { JoinCodeDialog } from "./JoinCodeDialog";
import { RoleChip } from "./RoleChip";
import { TeamRoster } from "./TeamRoster";

type Dialog = "leave" | "delete" | null;

function teamMark(name: string) {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0])
    .join("")
    .toLocaleUpperCase();
}

function TeamPage({ team }: { team: Team }) {
  const t = useTranslations("Teams");
  const tErr = useTranslations("errors");
  const locale = useLocale();
  const router = useRouter();
  const capabilities = deriveCapabilities(team.role);
  const leave = useLeaveTeam(team.team_id);
  const remove = useDeleteTeam(team.team_id);
  const [dialog, setDialog] = useState<Dialog>(null);
  const errorText = (error: Error | null) =>
    error ? (tErr.has(error.message) ? tErr(error.message) : t("actionFailed")) : null;
  const leaveOrDeleteDone = () => router.replace("/teams");

  return (
    <div className="space-y-8">
      <IdentityHeader
        mark={teamMark(team.name)}
        title={team.name}
        subtitle={
          <div className="flex flex-wrap items-center gap-2">
            <RoleChip role={team.role} />
            <span aria-hidden="true">·</span>
            <span>
              {t("createdOn", {
                date: new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(
                  new Date(team.created_at),
                ),
              })}
            </span>
          </div>
        }
      />

      <section id="members" aria-labelledby="team-members" className="scroll-mt-24 space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h2 id="team-members" className="text-text font-semibold">
            {t("membersWithCount", { count: team.member_count })}
          </h2>
          {capabilities.canViewInvitationCode ? <JoinCodeDialog teamId={team.team_id} /> : null}
        </div>
        <TeamRoster team={team} />
      </section>

      <section aria-labelledby="team-danger" className="border-sev-critical/40 border-t pt-6">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <h2 id="team-danger" className="text-sev-critical font-semibold">
              {t("danger")}
            </h2>
            <p className="text-muted mt-1 text-sm">
              {capabilities.canDeleteTeam ? t("deleteTeamWarning") : t("leaveTeamWarning")}
            </p>
          </div>
          {capabilities.canDeleteTeam ? (
            <Button
              variant="danger"
              onClick={() => {
                remove.reset();
                setDialog("delete");
              }}
            >
              {t("deleteTeam")}
            </Button>
          ) : (
            <Button
              onClick={() => {
                leave.reset();
                setDialog("leave");
              }}
            >
              {t("leaveTeam")}
            </Button>
          )}
        </div>
      </section>

      <ConfirmDialog
        open={dialog === "leave"}
        title={t("leaveTeam")}
        description={t("leaveConfirm", { name: team.name })}
        confirmLabel={t("leaveTeam")}
        cancelLabel={t("cancel")}
        intent="destructive"
        pendingLabel={t("processing")}
        pending={leave.isPending}
        error={errorText(leave.error)}
        onConfirm={() => leave.mutate(undefined, { onSuccess: leaveOrDeleteDone })}
        onClose={() => setDialog(null)}
      />
      <ConfirmDialog
        open={dialog === "delete"}
        title={t("deleteTeam")}
        description={t("deleteConfirm", { name: team.name })}
        confirmLabel={t("deleteTeam")}
        cancelLabel={t("cancel")}
        intent="destructive"
        pendingLabel={t("processing")}
        requireType="DELETE"
        pending={remove.isPending}
        error={errorText(remove.error)}
        onConfirm={() => remove.mutate(undefined, { onSuccess: leaveOrDeleteDone })}
        onClose={() => setDialog(null)}
      />
    </div>
  );
}

export function TeamSettingsPage({ teamId }: { teamId: string }) {
  const t = useTranslations("Teams");
  const { data: teams, isLoading, error } = useTeams();
  const team = teams?.find((candidate) => candidate.team_id === teamId);
  const state: PageContentState = isLoading ? "loading" : error || !team ? "error" : "ready";

  return (
    <PageLayout>
      <PageContent
        state={state}
        loadingFallback={
          <div className="text-muted animate-pulse py-12 text-center">{t("loading")}</div>
        }
        errorFallback={<Alert tone="danger">{t("teamUnavailable")}</Alert>}
      >
        {team ? <TeamPage team={team} /> : null}
      </PageContent>
    </PageLayout>
  );
}
